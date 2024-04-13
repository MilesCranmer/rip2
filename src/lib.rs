use chrono::Local;
use clap::CommandFactory;
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
pub const BIG_FILE_THRESHOLD: u64 = 500000000; // 500 MB

pub struct RecordItem<'a> {
    _time: &'a str,
    orig: &'a Path,
    dest: &'a Path,
}

pub fn run(cli: Args, mode: impl util::TestingMode, stream: &mut impl Write) -> Result<(), Error> {
    args::validate_args(&cli)?;
    // This selects the location of deleted
    // files based on the following order (from
    // first choice to last):
    // 1. Path passed with --graveyard
    // 2. Path pointed by the $GRAVEYARD variable
    // 3. $XDG_DATA_HOME/graveyard (only if XDG_DATA_HOME is defined)
    // 4. /tmp/graveyard-user
    let graveyard: &PathBuf = &{
        if let Some(flag) = cli.graveyard {
            flag
        } else if let Ok(env) = env::var("RIP_GRAVEYARD") {
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
        fs::create_dir_all(graveyard)?;
        let metadata = graveyard.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o700);
    }

    // If the user wishes to restore everything
    if cli.decompose {
        if util::prompt_yes("Really unlink the entire graveyard?", &mode, stream)? {
            fs::remove_dir_all(graveyard)?;
        }
        return Ok(());
    }

    // Stores the deleted files
    let record: &Path = &graveyard.join(RECORD);
    let cwd = &env::current_dir()?;

    if let Some(mut graves_to_exhume) = cli.unbury {
        // Vector to hold the grave path of items we want to unbury.
        // This will be used to determine which items to remove from the
        // record following the unbury.
        // Initialize it with the targets passed to -r

        // If -s is also passed, push all files found by seance onto
        // the graves_to_exhume.
        if cli.seance {
            if let Ok(record_file) = fs::File::open(record) {
                let gravepath = util::join_absolute(graveyard, cwd)
                    .to_string_lossy()
                    .into_owned();
                for grave in seance(record_file, gravepath) {
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
        let record_file = fs::File::open(record)?;

        for line in lines_of_graves(record_file, &graves_to_exhume) {
            let entry: RecordItem = record_entry(&line);
            let orig: PathBuf = match util::symlink_exists(entry.orig) {
                true => util::rename_grave(entry.orig),
                false => PathBuf::from(entry.orig),
            };

            move_target(entry.dest, &orig, &mode, stream).map_err(|e| {
                Error::new(
                    e.kind(),
                    format!(
                        "Unbury failed: couldn't copy files from {} to {}",
                        entry.dest.display(),
                        orig.display()
                    ),
                )
            })?;
            writeln!(
                stream,
                "Returned {} to {}",
                entry.dest.display(),
                orig.display()
            )?;
        }

        // Reopen the record and then delete lines corresponding to exhumed graves
        fs::File::open(record)
            .and_then(|record_file| {
                delete_lines_from_record(record_file, record, &graves_to_exhume)
            })
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
        let record_file = fs::File::open(record)
            .map_err(|_| Error::new(ErrorKind::NotFound, "Failed to read record!"))?;
        for grave in seance(record_file, gravepath.to_string_lossy()) {
            writeln!(stream, "{}", grave.display())?;
        }
        return Ok(());
    }

    if cli.targets.is_empty() {
        Args::command().print_help()?;
        return Ok(());
    }

    for target in cli.targets {
        bury_target(&target, record, graveyard, cwd, cli.inspect, &mode, stream)?;
    }

    Ok(())
}

fn bury_target(
    target: &PathBuf,
    record: &Path,
    graveyard: &PathBuf,
    cwd: &Path,
    inspect: bool,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
) -> Result<(), Error> {
    // Check if source exists
    let metadata = fs::symlink_metadata(target).map_err(|_| {
        Error::new(
            ErrorKind::NotFound,
            format!(
                "Cannot remove {}: no such file or directory",
                target.to_str().unwrap()
            ),
        )
    })?;
    // Canonicalize the path unless it's a symlink
    let source = &if !metadata.file_type().is_symlink() {
        cwd.join(target)
            .canonicalize()
            .map_err(|e| Error::new(e.kind(), "Failed to canonicalize path"))?
    } else {
        cwd.join(target)
    };

    if inspect {
        let moved_to_graveyard = do_inspection(target, source, metadata, mode, stream)?;
        if moved_to_graveyard {
            return Ok(());
        }
    }

    // If rip is called on a file already in the graveyard, prompt
    // to permanently delete it instead.
    if source.starts_with(graveyard) {
        writeln!(stream, "{} is already in the graveyard.", source.display())?;
        if util::prompt_yes("Permanently unlink it?", mode, stream)? {
            if fs::remove_dir_all(source).is_err() {
                if let Err(e) = fs::remove_file(source) {
                    return Err(Error::new(e.kind(), "Couldn't unlink!"));
                }
            }
            return Ok(());
        } else {
            writeln!(stream, "Skipping {}", source.display())?;
            return Ok(());
        }
    }

    let dest: &Path = &{
        let dest = util::join_absolute(graveyard, source);
        // Resolve a name conflict if necessary
        if util::symlink_exists(&dest) {
            util::rename_grave(dest)
        } else {
            dest
        }
    };

    move_target(source, dest, mode, stream).map_err(|e| {
        fs::remove_dir_all(dest).ok();
        Error::new(e.kind(), "Failed to bury file")
    })?;

    // Clean up any partial buries due to permission error
    write_log(source, dest, record).map_err(|e| {
        Error::new(
            e.kind(),
            format!("Failed to write record at {}", record.display()),
        )
    })?;

    Ok(())
}

fn do_inspection(
    target: &Path,
    source: &PathBuf,
    metadata: Metadata,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
) -> Result<bool, Error> {
    if metadata.is_dir() {
        // Get the size of the directory and all its contents
        writeln!(
            stream,
            "{}: directory, {} including:",
            target.to_str().unwrap(),
            util::humanize_bytes(
                WalkDir::new(source)
                    .into_iter()
                    .filter_map(|x| x.ok())
                    .filter_map(|x| x.metadata().ok())
                    .map(|x| x.len())
                    .sum::<u64>(),
            )
        )?;

        // Print the first few top-level files in the directory
        for entry in WalkDir::new(source)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .take(FILES_TO_INSPECT)
        {
            writeln!(stream, "{}", entry.path().display())?;
        }
    } else {
        writeln!(
            stream,
            "{}: file, {}",
            &target.to_str().unwrap(),
            util::humanize_bytes(metadata.len())
        )?;
        // Read the file and print the first few lines
        if let Ok(source_file) = fs::File::open(source) {
            for line in BufReader::new(source_file)
                .lines()
                .take(LINES_TO_INSPECT)
                .filter_map(|line| line.ok())
            {
                writeln!(stream, "> {}", line)?;
            }
        } else {
            writeln!(stream, "Error reading {}", source.display())?;
        }
    }
    Ok(!util::prompt_yes(
        format!("Send {} to the graveyard?", target.to_str().unwrap()),
        mode,
        stream,
    )?)
}

/// Write deletion history to record
fn write_log(
    source: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    record: impl AsRef<Path>,
) -> io::Result<()> {
    let (source, dest) = (source.as_ref(), dest.as_ref());
    let mut record_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(record)?;
    writeln!(
        record_file,
        "{}\t{}\t{}",
        Local::now().to_rfc3339(),
        source.display(),
        dest.display()
    )?;

    Ok(())
}

pub fn move_target(
    target: &Path,
    dest: &Path,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
) -> Result<(), Error> {
    // Try a simple rename, which will only work within the same mount point.
    // Trying to rename across filesystems will throw errno 18.
    if fs::rename(target, dest).is_ok() {
        return Ok(());
    }

    // If that didn't work, then copy and rm.
    {
        let parent = dest
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Could not get parent of dest!"))?;

        fs::create_dir_all(parent)?
    }

    let sym_link_data = fs::symlink_metadata(target)?;
    if sym_link_data.is_dir() {
        // Walk the source, creating directories and copying files as needed
        for entry in WalkDir::new(target).into_iter().filter_map(|e| e.ok()) {
            // Path without the top-level directory
            let orphan = entry.path().strip_prefix(target).map_err(|_| {
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
                copy_file(entry.path(), &dest.join(orphan), mode, stream).map_err(|e| {
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
        fs::remove_dir_all(target).map_err(|e| {
            Error::new(
                e.kind(),
                format!("Failed to remove dir: {}", target.display()),
            )
        })?;
    } else {
        copy_file(target, dest, mode, stream).map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "Failed to copy file from {} to {}",
                    target.display(),
                    dest.display()
                ),
            )
        })?;
        fs::remove_file(target).map_err(|e| {
            Error::new(
                e.kind(),
                format!("Failed to remove file: {}", target.display()),
            )
        })?;
    }

    Ok(())
}

pub fn copy_file(
    source: &Path,
    dest: &Path,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
) -> Result<(), Error> {
    let metadata = fs::symlink_metadata(source)?;
    let filetype = metadata.file_type();

    if metadata.len() > BIG_FILE_THRESHOLD {
        writeln!(
            stream,
            "About to copy a big file ({} is {})",
            source.display(),
            util::humanize_bytes(metadata.len())
        )?;
        if util::prompt_yes("Permanently delete this file instead?", mode, stream)? {
            return Ok(());
        }
    }

    if filetype.is_file() {
        fs::copy(source, dest)?;
    } else if filetype.is_fifo() {
        let metadata_mode = metadata.permissions().mode();
        std::process::Command::new("mkfifo")
            .arg(dest)
            .arg("-m")
            .arg(metadata_mode.to_string())
            .output()?;
    } else if filetype.is_symlink() {
        let target = fs::read_link(source)?;
        std::os::unix::fs::symlink(target, dest)?;
    } else if let Err(e) = fs::copy(source, dest) {
        // Special file: Try copying it as normal, but this probably won't work
        writeln!(
            stream,
            "Non-regular file or directory: {}",
            source.display()
        )?;
        if !util::prompt_yes("Permanently delete the file?", mode, stream)? {
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
fn get_last_bury(record: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let record_file = fs::File::open(record.as_ref())?;
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
                delete_lines_from_record(record_file, record, &graves_to_exhume)?;
            }
            return Ok(PathBuf::from(entry.dest));
        } else {
            // File is gone, mark the grave to be removed from the record
            graves_to_exhume.push(PathBuf::from(entry.dest));
        }
    }

    if !graves_to_exhume.is_empty() {
        delete_lines_from_record(record_file, record, &graves_to_exhume)?;
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
fn lines_of_graves(record_file: fs::File, graves: &[PathBuf]) -> impl Iterator<Item = String> + '_ {
    BufReader::new(record_file)
        .lines()
        .map_while(Result::ok)
        .filter(move |line| graves.iter().any(|y| y == record_entry(line).dest))
}

/// Returns an iterator over all graves in the record that are under gravepath
fn seance<T: AsRef<str>>(record_file: fs::File, gravepath: T) -> impl Iterator<Item = PathBuf> {
    BufReader::new(record_file)
        .lines()
        .map_while(Result::ok)
        .map(|line| PathBuf::from(record_entry(&line).dest))
        .filter(move |d| d.starts_with(gravepath.as_ref()))
}

/// Takes a vector of grave paths and removes the respective lines from the record
fn delete_lines_from_record(
    record_file: fs::File,
    record: impl AsRef<Path>,
    graves: &[PathBuf],
) -> Result<(), Error> {
    let record = record.as_ref();
    // Get the lines to write back to the record, which is every line except
    // the ones matching the exhumed graves.  Store them in a vector
    // since we'll be overwriting the record in-place.
    let lines_to_write: Vec<String> = BufReader::new(record_file)
        .lines()
        .map_while(Result::ok)
        .filter(|line| !graves.iter().any(|y| y == record_entry(line).dest))
        .collect();
    let mut mutable_record_file = fs::File::create(record)?;
    for line in lines_to_write {
        writeln!(mutable_record_file, "{}", line)?;
    }
    Ok(())
}
