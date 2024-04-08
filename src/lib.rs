use clap::CommandFactory;
use chrono::Local;
use std::fs::Metadata;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::{env, fs, io};
use walkdir::WalkDir;

pub mod args;
pub mod util;

use args::Args;

const GRAVEYARD: &str = "/tmp/graveyard";
const RECORD: &str = ".record";
const LINES_TO_INSPECT: usize = 6;
const FILES_TO_INSPECT: usize = 6;
const BIG_FILE_THRESHOLD: u64 = 500000000; // 500 MB

pub struct RecordItem<'a> {
    _time: &'a str,
    orig: &'a Path,
    dest: &'a Path,
}

pub fn run<M: util::TestingMode>(cli: Args, mode: M) -> Result<(), Error> {
    args::validate_args(&cli)?;
    // This selects the location of deleted
    // files based on the following order (from
    // first choice to last):
    // 1. Path passed with --graveyard
    // 2. Path pointed by the $GRAVEYARD variable
    // 3. $XDG_DATA_HOME/graveyard (only if XDG_DATA_HOME is defined)
    // 4. /tmp/graveyard-user
    let graveyard: PathBuf = {
        if let Some(flag) = cli.graveyard {
            flag
        } else if let Ok(env) = env::var("GRAVEYARD") {
            PathBuf::from(env)
        } else if let Ok(mut env) = env::var("XDG_DATA_HOME") {
            if !env.ends_with(std::path::MAIN_SEPARATOR) {
                env.push(std::path::MAIN_SEPARATOR);
            }
            env.push_str("graveyard");
            PathBuf::from(env)
        } else {
            PathBuf::from(format!("{}-{}", GRAVEYARD, util::get_user()))
        }
    };

    if !graveyard.exists() {
        fs::create_dir_all(&graveyard)?;
        let metadata = graveyard.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o700);
    }

    // If the user wishes to restore everything
    if cli.decompose {
        if util::prompt_yes("Really unlink the entire graveyard?", &mode) {
            fs::remove_dir_all(graveyard)?;
        }
        return Ok(());
    }

    // Stores the deleted files
    let record: &Path = &graveyard.join(RECORD);
    let cwd = env::current_dir()?;

    if let Some(t) = cli.unbury {
        // Vector to hold the grave path of items we want to unbury.
        // This will be used to determine which items to remove from the
        // record following the unbury.
        // Initialize it with the targets passed to -r
        let mut graves_to_exhume: Vec<PathBuf> = t.iter().map(PathBuf::from).collect();

        // If -s is also passed, push all files found by seance onto
        // the graves_to_exhume.
        if cli.seance {
            if let Ok(f) = fs::File::open(record) {
                let gravepath = util::join_absolute(graveyard, cwd)
                    .to_string_lossy()
                    .into_owned();
                for grave in seance(f, gravepath) {
                    graves_to_exhume.push(grave);
                }
            }
        }

        // Otherwise, add the last deleted file
        if graves_to_exhume.is_empty() {
            if let Ok(s) = get_last_bury(record) {
                graves_to_exhume.push(s);
            }
        }

        // Go through the graveyard and exhume all the graves
        let f = fs::File::open(record)?;

        for line in lines_of_graves(f, &graves_to_exhume) {
            let entry: RecordItem = record_entry(&line);
            let orig: PathBuf = match util::symlink_exists(entry.orig) {
                true => util::rename_grave(entry.orig),
                false => PathBuf::from(entry.orig),
            };

            bury(entry.dest, &orig, &mode).map_err(|e| {
                Error::new(
                    e.kind(),
                    format!(
                        "Unbury failed: couldn't copy files from {} to {}",
                        entry.dest.display(),
                        orig.display()
                    ),
                )
            })?;
            println!("Returned {} to {}", entry.dest.display(), orig.display());
        }

        // Reopen the record and then delete lines corresponding to exhumed graves
        fs::File::open(record)
            .and_then(|f| delete_lines_from_record(f, record, &graves_to_exhume))
            .map_err(|e| {
                Error::new(
                    e.kind(),
                    format!("Failed to remove unburied files from record: {}", e),
                )
            })?;
        return Ok(());
    }

    if cli.seance {
        let gravepath = util::join_absolute(graveyard, cwd);
        let f = fs::File::open(record)
            .map_err(|_| Error::new(ErrorKind::NotFound, "Failed to read record!"))?;
        for grave in seance(f, gravepath.to_string_lossy()) {
            println!("{}", grave.display());
        }
        return Ok(());
    }

    if !cli.targets.is_empty() {
        for target in cli.targets {
            // Check if source exists
            if let Ok(metadata) = fs::symlink_metadata(&target) {
                // Canonicalize the path unless it's a symlink
                let source = &if !metadata.file_type().is_symlink() {
                    cwd.join(&target)
                        .canonicalize()
                        .map_err(|e| Error::new(e.kind(), "Failed to canonicalize path"))?
                } else {
                    cwd.join(&target)
                };

                if cli.inspect {
                    let moved_to_graveyard = do_inspection(target, source, metadata, &mode);
                    if moved_to_graveyard {
                        continue;
                    }
                }

                // If rip is called on a file already in the graveyard, prompt
                // to permanently delete it instead.
                if source.starts_with(&graveyard) {
                    println!("{} is already in the graveyard.", source.display());
                    if util::prompt_yes("Permanently unlink it?", &mode) {
                        if fs::remove_dir_all(source).is_err() {
                            if let Err(e) = fs::remove_file(source) {
                                return Err(Error::new(e.kind(), "Couldn't unlink!"));
                            }
                        }
                        continue;
                    } else {
                        println!("Skipping {}", source.display());
                        return Ok(());
                    }
                }

                let dest: &Path = &{
                    let dest = util::join_absolute(&graveyard, source);
                    // Resolve a name conflict if necessary
                    if util::symlink_exists(&dest) {
                        util::rename_grave(dest)
                    } else {
                        dest
                    }
                };

                {
                    let res = bury(source, dest, &mode).map_err(|e| {
                        fs::remove_dir_all(dest).ok();
                        e
                    });
                    if let Err(e) = res {
                        return Err(Error::new(e.kind(), "Failed to bury file"));
                    }
                }
                // Clean up any partial buries due to permission error
                if let Err(e) = write_log(source, dest, record) {
                    return Err(Error::new(
                        e.kind(),
                        format!("Failed to write record at {}", record.display()),
                    ));
                }
            } else {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!(
                        "Cannot remove {}: no such file or directory",
                        target.to_str().unwrap()
                    ),
                ));
            }
        }
    } else {
        let _ = Args::command().print_help();
    }

    Ok(())
}

fn do_inspection<M: util::TestingMode>(
    target: PathBuf,
    source: &PathBuf,
    metadata: Metadata,
    mode: &M,
) -> bool {
    if metadata.is_dir() {
        // Get the size of the directory and all its contents
        println!(
            "{}: directory, {} including:",
            target.to_str().unwrap(),
            util::humanize_bytes(
                WalkDir::new(source)
                    .into_iter()
                    .filter_map(|x| x.ok())
                    .filter_map(|x| x.metadata().ok())
                    .map(|x| x.len())
                    .sum::<u64>()
            )
        );

        // Print the first few top-level files in the directory
        for entry in WalkDir::new(source)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .take(FILES_TO_INSPECT)
        {
            println!("{}", entry.path().display());
        }
    } else {
        println!(
            "{}: file, {}",
            &target.to_str().unwrap(),
            util::humanize_bytes(metadata.len())
        );
        // Read the file and print the first few lines
        if let Ok(f) = fs::File::open(source) {
            for line in BufReader::new(f)
                .lines()
                .take(LINES_TO_INSPECT)
                .filter_map(|line| line.ok())
            {
                println!("> {}", line);
            }
        } else {
            println!("Error reading {}", source.display());
        }
    }
    !util::prompt_yes(
        format!("Send {} to the graveyard?", target.to_str().unwrap()),
        mode,
    )
}

/// Write deletion history to record
fn write_log<S, D, R>(source: S, dest: D, record: R) -> io::Result<()>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    R: AsRef<Path>,
{
    let (source, dest) = (source.as_ref(), dest.as_ref());
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(record)?;
    writeln!(
        f,
        "{}\t{}\t{}",
        Local::now().to_rfc3339(),
        source.display(),
        dest.display()
    )?;

    Ok(())
}

pub fn bury<S, D, M>(source: S, dest: D, mode: &M) -> Result<(), Error>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    M: util::TestingMode,
{
    let (source, dest) = (source.as_ref(), dest.as_ref());
    // Try a simple rename, which will only work within the same mount point.
    // Trying to rename across filesystems will throw errno 18.
    if fs::rename(source, dest).is_ok() {
        return Ok(());
    }

    // If that didn't work, then copy and rm.
    {
        let parent = dest.parent();
        if parent.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Could not get parent of dest!",
            ));
        }

        let parent = parent.unwrap();
        fs::create_dir_all(parent)?
    }

    let sym_link_data = fs::symlink_metadata(source)?;
    if sym_link_data.is_dir() {
        // Walk the source, creating directories and copying files as needed
        for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            // Path without the top-level directory
            let orphan = entry.path().strip_prefix(source).map_err(|_| {
                Error::new(
                    ErrorKind::Other,
                    "Parent directory isn't a prefix of child directories?",
                )
            })?;

            if entry.file_type().is_dir() {
                fs::create_dir_all(dest.join(orphan)).map_err(|e| {
                    Error::new(
                        e.kind(),
                        format!(
                            "Failed to create dir: {} in {}",
                            entry.path().display(),
                            dest.join(orphan).display()
                        ),
                    )
                })?;
            } else {
                copy_file(entry.path(), dest.join(orphan), mode).map_err(|e| {
                    Error::new(
                        e.kind(),
                        format!(
                            "Failed to copy file from {} to {}",
                            entry.path().display(),
                            dest.join(orphan).display()
                        ),
                    )
                })?;
            }
        }
        fs::remove_dir_all(source).map_err(|e| {
            Error::new(
                e.kind(),
                format!("Failed to remove dir: {}", source.display()),
            )
        })?;
    } else {
        copy_file(source, dest, mode).map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "Failed to copy file from {} to {}",
                    source.display(),
                    dest.display()
                ),
            )
        })?;
        fs::remove_file(source).map_err(|e| {
            Error::new(
                e.kind(),
                format!("Failed to remove file: {}", source.display()),
            )
        })?;
    }

    Ok(())
}

fn copy_file<S, D, M>(source: S, dest: D, mode: &M) -> Result<(), Error>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    M: util::TestingMode,
{
    let (source, dest) = (source.as_ref(), dest.as_ref());
    let metadata = fs::symlink_metadata(source)?;
    let filetype = metadata.file_type();

    if metadata.len() > BIG_FILE_THRESHOLD {
        println!(
            "About to copy a big file ({} is {})",
            source.display(),
            util::humanize_bytes(metadata.len())
        );
        if util::prompt_yes("Permanently delete this file instead?", mode) {
            return Ok(());
        }
    }

    if filetype.is_file() {
        fs::copy(source, dest)?;
    } else if filetype.is_fifo() {
        let mode = metadata.permissions().mode();
        std::process::Command::new("mkfifo")
            .arg(dest)
            .arg("-m")
            .arg(mode.to_string());
    } else if filetype.is_symlink() {
        let target = fs::read_link(source)?;
        std::os::unix::fs::symlink(target, dest)?;
    } else if let Err(e) = fs::copy(source, dest) {
        // Special file: Try copying it as normal, but this probably won't work
        println!("Non-regular file or directory: {}", source.display());
        if !util::prompt_yes("Permanently delete the file?", mode) {
            return Err(e);
        }
        // Create a dummy file to act as a marker in the graveyard
        let mut marker = fs::File::create(dest)?;
        marker.write_all(
            b"This is a marker for a file that was \
                           permanently deleted.  Requiescat in pace.",
        )?;
    }

    Ok(())
}

/// Return the path in the graveyard of the last file to be buried.
/// As a side effect, any valid last files that are found in the record but
/// not on the filesystem are removed from the record.
fn get_last_bury<R: AsRef<Path>>(record: R) -> Result<PathBuf, Error> {
    let f = fs::File::open(record.as_ref())?;
    let contents = {
        let path_f = PathBuf::from(record.as_ref());
        fs::read_to_string(path_f)?
    };

    // This will be None if there is nothing, or Some
    // if there is items in the vector
    let mut graves_to_exhume: Vec<PathBuf> = Vec::new();
    for entry in contents.lines().rev().map(record_entry) {
        // Check that the file is still in the graveyard.
        // If it is, return the corresponding line.
        if util::symlink_exists(entry.dest) {
            if !graves_to_exhume.is_empty() {
                delete_lines_from_record(f, record, &graves_to_exhume)?;
            }
            return Ok(PathBuf::from(entry.dest));
        } else {
            // File is gone, mark the grave to be removed from the record
            graves_to_exhume.push(PathBuf::from(entry.dest));
        }
    }

    if !graves_to_exhume.is_empty() {
        delete_lines_from_record(f, record, &graves_to_exhume)?;
    }
    Err(Error::new(ErrorKind::NotFound, "No files in graveyard"))
}

/// Parse a line in the record into a `RecordItem`
fn record_entry(line: &str) -> RecordItem {
    let mut tokens = line.split('\t');
    let time: &str = tokens.next().expect("Bad format: column A");
    let orig: &str = tokens.next().expect("Bad format: column B");
    let dest: &str = tokens.next().expect("Bad format: column C");
    RecordItem {
        _time: time,
        orig: Path::new(orig),
        dest: Path::new(dest),
    }
}

/// Takes a vector of grave paths and returns the respective lines in the record
fn lines_of_graves(f: fs::File, graves: &[PathBuf]) -> impl Iterator<Item = String> + '_ {
    BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter(move |l| graves.iter().any(|y| y == record_entry(l).dest))
}

/// Returns an iterator over all graves in the record that are under gravepath
fn seance<T: AsRef<str>>(f: fs::File, gravepath: T) -> impl Iterator<Item = PathBuf> {
    BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .map(|l| PathBuf::from(record_entry(&l).dest))
        .filter(move |d| d.starts_with(gravepath.as_ref()))
}

/// Takes a vector of grave paths and removes the respective lines from the record
fn delete_lines_from_record<R: AsRef<Path>>(
    current_record: fs::File,
    record: R,
    graves: &[PathBuf],
) -> Result<(), Error> {
    let record = record.as_ref();
    // Get the lines to write back to the record, which is every line except
    // the ones matching the exhumed graves.  Store them in a vector
    // since we'll be overwriting the record in-place.
    let lines_to_write: Vec<String> = BufReader::new(current_record)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !graves.iter().any(|y| y == record_entry(l).dest))
        .collect();
    let mut f = fs::File::create(record)?;
    // if let Err(err) = f {
    //     return Err(err);
    // }
    for line in lines_to_write {
        writeln!(f, "{}", line)?;
    }
    Ok(())
}
