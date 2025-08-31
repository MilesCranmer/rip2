use clap::CommandFactory;
use fs_extra::dir::get_size;
use std::fs::Metadata;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};
use walkdir::WalkDir;

// Platform-specific imports
#[cfg(unix)]
use nix::libc;
#[cfg(unix)]
use nix::sys::stat::Mode;
#[cfg(unix)]
use nix::unistd::mkfifo;
#[cfg(unix)]
use std::os::unix::fs::{symlink, FileTypeExt, PermissionsExt};

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;

pub mod args;
pub mod completions;
pub mod record;
pub mod util;

use args::Args;
use record::{Record, RecordItem, DEFAULT_FILE_LOCK};

const LINES_TO_INSPECT: usize = 6;
const FILES_TO_INSPECT: usize = 6;
pub const BIG_FILE_THRESHOLD: u64 = 500_000_000; // 500 MB

pub fn run(cli: &Args, mode: impl util::TestingMode, stream: &mut impl Write) -> Result<(), Error> {
    args::validate_args(cli)?;
    let graveyard: &PathBuf = &get_graveyard(cli.graveyard.clone());

    if !graveyard.exists() {
        fs::create_dir_all(graveyard)?;

        #[cfg(unix)]
        {
            let metadata = graveyard.metadata()?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
        }
        // TODO: Default permissions on windows should be good, but need to double-check.
    }

    // Stores the deleted files
    let record = Record::<DEFAULT_FILE_LOCK>::new(graveyard);
    let cwd = &env::current_dir()?;

    // If the user wishes to restore everything
    if cli.decompose {
        // In force mode, skip the prompt to decompose
        if cli.force || util::prompt_yes("Really unlink the entire graveyard?", &mode, stream)? {
            fs::remove_dir_all(graveyard)?;
        }
    } else if let Some(ref mut graves_to_exhume) = cli.unbury.clone() {
        // Vector to hold the grave path of items we want to unbury.
        // This will be used to determine which items to remove from the
        // record following the unbury.
        // Initialize it with the targets passed to -r

        // If -s is also passed, push all files found by seance onto
        // the graves_to_exhume.
        if cli.seance && record.open().is_ok() {
            let gravepath = util::join_absolute(graveyard, dunce::canonicalize(cwd)?);
            for grave in record.seance(&gravepath)? {
                graves_to_exhume.push(grave.dest);
            }
        }

        // Otherwise, add the last deleted file
        if graves_to_exhume.is_empty() {
            if let Ok(s) = record.get_last_bury() {
                graves_to_exhume.push(s);
            }
        }

        let allow_rename = util::allow_rename();

        // Go through the graveyard and exhume all the graves
        for line in record.lines_of_graves(graves_to_exhume) {
            let entry = RecordItem::new(&line);
            let orig: PathBuf = if util::symlink_exists(&entry.orig) {
                util::rename_grave(&entry.orig)
            } else {
                PathBuf::from(&entry.orig)
            };
            move_target(&entry.dest, &orig, allow_rename, &mode, stream, cli.force).map_err(
                |e| {
                    Error::new(
                        e.kind(),
                        format!(
                            "Unbury failed: couldn't copy files from {} to {}",
                            entry.dest.display(),
                            orig.display()
                        ),
                    )
                },
            )?;
            writeln!(
                stream,
                "Returned {} to {}",
                entry.dest.display(),
                orig.display()
            )?;
        }
        record.log_exhumed_graves(graves_to_exhume)?;
    } else if cli.seance {
        let gravepath = util::join_absolute(graveyard, dunce::canonicalize(cwd)?);
        writeln!(stream, "{: <19}\tpath", "deletion_time")?;
        for grave in record.seance(&gravepath)? {
            let formatted_time = grave.format_time_for_display()?;
            writeln!(stream, "{}\t{}", formatted_time, grave.dest.display())?;
        }
    } else if cli.targets.is_empty() {
        Args::command().print_help()?;
    } else {
        let allow_rename = util::allow_rename();
        for target in &cli.targets {
            bury_target(
                target,
                graveyard,
                &record,
                cwd,
                cli.inspect,
                allow_rename,
                &mode,
                stream,
                cli.force,
            )?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn bury_target<const FILE_LOCK: bool>(
    target: &PathBuf,
    graveyard: &PathBuf,
    record: &Record<FILE_LOCK>,
    cwd: &Path,
    inspect: bool,
    allow_rename: bool,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
    force: bool,
) -> Result<(), Error> {
    // Check if source exists
    let metadata = &fs::symlink_metadata(target).map_err(|_| {
        Error::new(
            ErrorKind::NotFound,
            format!(
                "Cannot remove {}: no such file or directory",
                target.to_str().unwrap()
            ),
        )
    })?;
    // Canonicalize the path unless it's a symlink
    let source = &if metadata.file_type().is_symlink() {
        cwd.join(target)
    } else {
        dunce::canonicalize(cwd.join(target))
            .map_err(|e| Error::new(e.kind(), "Failed to canonicalize path"))?
    };

    if inspect && !should_we_bury_this(target, source, metadata, mode, stream)? {
        // User chose to not bury the file
    } else if source.starts_with(
        dunce::canonicalize(graveyard)
            .map_err(|e| Error::new(e.kind(), "Failed to canonicalize graveyard path"))?,
    ) {
        // If rip is called on a file already in the graveyard, prompt
        // to permanently delete it instead.
        if force
            || util::prompt_yes(
                format!(
                    "{} is already in the graveyard.\nPermanently unlink it?",
                    source.display()
                ),
                mode,
                stream,
            )?
        {
            if fs::remove_dir_all(source).is_err() {
                fs::remove_file(source).map_err(|e| {
                    Error::new(e.kind(), format!("Couldn't unlink {}", source.display()))
                })?;
            }
        } else {
            writeln!(stream, "Skipping {}", source.display())?;
            // TODO: In the original code, this was a hard return from the entire
            // method (i.e., `run`). I think it should just be a return from the bury
            // (meaning a `continue` in the original code's loop). But I'm not sure.
        }
    } else {
        let dest: &Path = &{
            let dest = util::join_absolute(graveyard, source);
            // Resolve a name conflict if necessary
            if util::symlink_exists(&dest) {
                util::rename_grave(dest)
            } else {
                dest
            }
        };

        let moved = move_target(source, dest, allow_rename, mode, stream, force).map_err(|e| {
            fs::remove_dir_all(dest).ok();
            Error::new(e.kind(), "Failed to bury file")
        })?;

        if moved {
            // Clean up any partial buries due to permission error
            record.write_log(source, dest)?;
        }
    }

    Ok(())
}

fn should_we_bury_this(
    target: &Path,
    source: &PathBuf,
    metadata: &Metadata,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
) -> Result<bool, Error> {
    if metadata.is_dir() {
        // Get the size of the directory and all its contents
        {
            let num_bytes = get_size(source).map_err(|_| {
                Error::other(format!(
                    "Failed to get size of directory: {}",
                    source.display()
                ))
            })?;
            writeln!(
                stream,
                "{}: directory, {} including:",
                target.to_str().unwrap(),
                util::humanize_bytes(num_bytes)
            )?;
        }

        // Print the first few top-level files in the directory
        for entry in WalkDir::new(source)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
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
                .filter_map(Result::ok)
            {
                writeln!(stream, "> {line}")?;
            }
        } else {
            writeln!(stream, "Error reading {}", source.display())?;
        }
    }
    util::prompt_yes(
        format!("Send {} to the graveyard?", target.to_str().unwrap()),
        mode,
        stream,
    )
}

/// Create directories preserving permissions from the source path
fn create_dirs_with_permissions(source: &Path, dest: &Path) -> Result<(), Error> {
    // Get the common prefix and the parts that need to be created
    let dest_parent = dest
        .parent()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "Could not get parent of dest!"))?;

    // First create all directories
    fs::create_dir_all(dest_parent)?;

    // Now walk the source path and apply permissions to corresponding dest directories
    let source_parent = source.parent();
    if let Some(src_parent) = source_parent {
        // Build up the path components from source
        let mut src_components = Vec::new();
        let mut current = src_parent;
        while let Some(parent) = current.parent() {
            src_components.push(current);
            current = parent;
        }
        src_components.push(current); // Add the root
        src_components.reverse();

        // Build up the path components from dest
        let mut dest_components = Vec::new();
        let mut current = dest_parent;
        while let Some(parent) = current.parent() {
            dest_components.push(current);
            current = parent;
        }
        dest_components.push(current); // Add the root
        dest_components.reverse();

        // Apply permissions where we have matching source paths
        let src_len = src_components.len();
        let dest_len = dest_components.len();
        if dest_len >= src_len {
            let offset = dest_len - src_len;
            for i in 0..src_len {
                if let Ok(src_meta) = fs::metadata(src_components[i]) {
                    if src_meta.is_dir() {
                        let dest_dir = dest_components[i + offset];
                        // Only set permissions if the directory exists (which it should after create_dir_all)
                        if dest_dir.exists() {
                            fs::set_permissions(dest_dir, src_meta.permissions()).map_err(|e| {
                                Error::new(
                                    e.kind(),
                                    format!(
                                        "Failed to preserve permissions on directory '{}': {}. The directory may be owned by another user.",
                                        dest_dir.display(),
                                        e
                                    ),
                                )
                            })?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Move a target to a given destination, copying if necessary.
/// Returns true if the target was moved, false if it was not (due to
/// user input)
pub fn move_target(
    target: &Path,
    dest: &Path,
    allow_rename: bool,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
    force: bool,
) -> Result<bool, Error> {
    // Try a simple rename, which will only work within the same mount point.
    // Trying to rename across filesystems will throw errno 18.
    if allow_rename && fs::rename(target, dest).is_ok() {
        return Ok(true);
    }

    // If that didn't work, then we need to copy and rm.
    create_dirs_with_permissions(target, dest)?;

    if fs::symlink_metadata(target)?.is_dir() {
        move_dir(target, dest, mode, stream, force)
    } else {
        let moved = copy_file(target, dest, mode, stream, force).map_err(|e| {
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
        Ok(moved)
    }
}

/// Move a target which is a directory to a given destination, copying if necessary.
/// Returns true *always*, as the creation of the directory is enough to mark it as successful.
pub fn move_dir(
    target: &Path,
    dest: &Path,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
    force: bool,
) -> Result<bool, Error> {
    // Walk the source, creating directories and copying files as needed
    for entry in WalkDir::new(target).into_iter().filter_map(Result::ok) {
        // Path without the top-level directory
        let orphan = entry
            .path()
            .strip_prefix(target)
            .map_err(|_| Error::other("Parent directory isn't a prefix of child directories?"))?;

        if entry.file_type().is_dir() {
            let dest_dir = dest.join(orphan);
            fs::create_dir_all(&dest_dir).map_err(|e| {
                Error::new(
                    e.kind(),
                    format!(
                        "Failed to create dir: {} in {}",
                        entry.path().display(),
                        dest_dir.display()
                    ),
                )
            })?;

            // Preserve directory permissions
            let source_metadata = fs::metadata(entry.path()).map_err(|e| {
                Error::new(
                    e.kind(),
                    format!("Failed to get metadata for: {}", entry.path().display()),
                )
            })?;
            let source_perms = source_metadata.permissions();
            fs::set_permissions(&dest_dir, source_perms).map_err(|e| {
                Error::new(
                    e.kind(),
                    format!("Failed to set permissions on: {}", dest_dir.display()),
                )
            })?;
        } else {
            copy_file(entry.path(), &dest.join(orphan), mode, stream, force).map_err(|e| {
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

    Ok(true)
}

pub fn copy_file(
    source: &Path,
    dest: &Path,
    mode: &impl util::TestingMode,
    stream: &mut impl Write,
    force: bool,
) -> Result<bool, Error> {
    let metadata = fs::symlink_metadata(source)?;
    let filetype = metadata.file_type();

    if metadata.len() > BIG_FILE_THRESHOLD {
        // In force mode, we default to copying big files
        if !force
            && util::prompt_yes(
                format!(
                    "About to copy a big file ({} is {})\nPermanently delete this file instead?",
                    source.display(),
                    util::humanize_bytes(metadata.len())
                ),
                mode,
                stream,
            )?
        {
            return Ok(false);
        }
    }

    if filetype.is_file() {
        fs::copy(source, dest)?;
        return Ok(true);
    }

    #[cfg(unix)]
    if filetype.is_fifo() {
        let perm: libc::mode_t = (metadata.permissions().mode() & 0o777) as libc::mode_t;
        let mode = Mode::from_bits_truncate(perm);

        mkfifo(dest, mode)?;
        return Ok(true);
    }

    if filetype.is_symlink() {
        let target = fs::read_link(source)?;
        symlink(target, dest)?;
        return Ok(true);
    }

    match fs::copy(source, dest) {
        Err(e) => {
            // Special file: Try copying it as normal, but this probably won't work
            // In force mode, we don't delete special files, we error
            if !force
                && util::prompt_yes(
                    format!(
                        "Non-regular file or directory: {}\nPermanently delete the file?",
                        source.display()
                    ),
                    mode,
                    stream,
                )?
            {
                Ok(false)
            } else {
                Err(e)
            }
        }
        Ok(_) => Ok(true),
    }
}

pub fn get_graveyard(graveyard: Option<PathBuf>) -> PathBuf {
    graveyard.map_or_else(
        || {
            if let Ok(env_graveyard) = env::var("RIP_GRAVEYARD") {
                PathBuf::from(env_graveyard)
            } else if let Ok(mut env_graveyard) = env::var("XDG_DATA_HOME") {
                if !env_graveyard.ends_with(std::path::MAIN_SEPARATOR) {
                    env_graveyard.push(std::path::MAIN_SEPARATOR);
                }
                env_graveyard.push_str("graveyard");
                PathBuf::from(env_graveyard)
            } else {
                let user = util::get_user();
                env::temp_dir().join(format!("graveyard-{user}"))
            }
        },
        |flag| flag,
    )
}

/// Testing module for exposing internal functions to unit tests.
/// This module is only used for testing purposes and should not be used in production code.
pub mod testing {
    use super::{should_we_bury_this, util, Error, Metadata, Path, PathBuf, Write};

    pub fn testable_should_we_bury_this(
        target: &Path,
        source: &PathBuf,
        metadata: &Metadata,
        stream: &mut impl Write,
    ) -> Result<bool, Error> {
        should_we_bury_this(target, source, metadata, &util::TestMode, stream)
    }
}
