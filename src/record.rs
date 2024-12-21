use chrono::{DateTime, Local};
use fs4::fs_std::FileExt;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::util;

pub const RECORD: &str = ".record";

#[derive(Debug)]
pub struct RecordItem {
    pub time: String,
    pub orig: PathBuf,
    pub dest: PathBuf,
}

impl RecordItem {
    /// Parse a line in the record into a `RecordItem`
    pub fn new(line: &str) -> RecordItem {
        let mut tokens = line.split('\t');
        let time = tokens.next().expect("Bad format: column 1").to_string();
        let orig = tokens.next().expect("Bad format: column 2").to_string();
        let dest = tokens.next().expect("Bad format: column 3").to_string();
        RecordItem {
            time,
            orig: PathBuf::from(orig),
            dest: PathBuf::from(dest),
        }
    }

    /// Parse the timestamp in this record, which could be in either RFC3339 format (from rip2)
    /// or the old rip format --- in which case we return a helpful error.
    fn parse_timestamp(&self) -> Result<DateTime<Local>, Error> {
        // Try parsing as RFC3339 first
        if let Ok(dt) = DateTime::parse_from_rfc3339(&self.time) {
            return Ok(dt.with_timezone(&Local));
        }

        // Roughly check if it matches the old rip format (e.g., "Sun Dec  1 02:15:56 2024")
        let is_old_format = self.time.split_whitespace().count() == 5
            && self
                .time
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || c == ':');
        if is_old_format {
            Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Found timestamp '{}' from old rip format. \
                    You will need to delete the `.record` file \
                    and start over with rip2. \
                    You can see the path with `rip graveyard`.",
                    self.time
                ),
            ))
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse time '{}' as RFC3339 format", self.time),
            ))
        }
    }

    /// Format this record's timestamp for display in the seance output
    pub fn format_time_for_display(&self) -> Result<String, Error> {
        self.parse_timestamp()
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
    }
}

/// A record of file operations maintained in the graveyard directory
///
/// # Type Parameters
///
/// * `FILE_LOCK` - When `true`, exclusive file locks are acquired when opening
///   the record file for reading or writing. This prevents concurrent access from multiple
///   processes. When `false`, no file locking is performed - which is used for testing.
#[derive(Debug)]
pub struct Record<const FILE_LOCK: bool> {
    path: PathBuf,
}

#[cfg(not(target_os = "windows"))]
pub const DEFAULT_FILE_LOCK: bool = true;

#[cfg(target_os = "windows")]
pub const DEFAULT_FILE_LOCK: bool = false;
// TODO: Investigate why this is needed. Does Windows not support file locks?

impl<const FILE_LOCK: bool> Record<FILE_LOCK> {
    pub fn new(graveyard: &Path) -> Record<FILE_LOCK> {
        let path = graveyard.join(RECORD);
        // Create the record file if it doesn't exist
        if !path.exists() {
            // Write a header to the record file
            let mut record_file = fs::OpenOptions::new()
                .truncate(true)
                .create(true)
                .write(true)
                .open(&path)
                .expect("Failed to open record file");
            if FILE_LOCK {
                record_file.lock_exclusive().unwrap();
            }
            record_file
                .write_all(b"Time\tOriginal\tDestination\n")
                .expect("Failed to write header to record file");
        }
        Record { path }
    }

    pub fn open(&self) -> Result<fs::File, Error> {
        let file = fs::File::open(&self.path)
            .map_err(|_| Error::new(ErrorKind::NotFound, "Failed to read record!"))?;
        if FILE_LOCK {
            file.lock_exclusive().unwrap();
        }
        Ok(file)
    }

    /// Return the path in the graveyard of the last file to be buried.
    /// As a side effect, any valid last files that are found in the record but
    /// not on the filesystem are removed from the record.
    pub fn get_last_bury(&self) -> Result<PathBuf, Error> {
        // record: impl AsRef<Path>
        let record_file = self.open()?;
        let mut contents = String::new();
        BufReader::new(&record_file).read_to_string(&mut contents)?;

        // This will be None if there is nothing, or Some
        // if there is items in the vector
        let mut graves_to_exhume: Vec<PathBuf> = Vec::new();
        let mut lines = contents.lines();
        lines.next();
        for entry in lines.rev().map(RecordItem::new) {
            // Check that the file is still in the graveyard.
            // If it is, return the corresponding line.
            if util::symlink_exists(&entry.dest) {
                if !graves_to_exhume.is_empty() {
                    self.delete_lines(record_file, &graves_to_exhume)?;
                }
                return Ok(entry.dest);
            } else {
                // File is gone, mark the grave to be removed from the record
                graves_to_exhume.push(entry.dest);
            }
        }

        if !graves_to_exhume.is_empty() {
            self.delete_lines(record_file, &graves_to_exhume)?;
        }
        Err(Error::new(ErrorKind::NotFound, "No files in graveyard"))
    }

    /// Takes a vector of grave paths and removes the respective lines from the record
    fn delete_lines(&self, record_file: fs::File, graves: &[PathBuf]) -> Result<(), Error> {
        let record_path = &self.path;
        // Get the lines to write back to the record, which is every line except
        // the ones matching the exhumed graves. Store them in a vector
        // since we'll be overwriting the record in-place.
        let mut reader = BufReader::new(record_file).lines();
        let header = reader
            .next()
            .unwrap_or_else(|| Ok(String::new()))
            .unwrap_or_default(); // Capture the header
        let lines_to_write: Vec<String> = reader
            .map_while(Result::ok)
            .filter(|line| !graves.iter().any(|y| *y == RecordItem::new(line).dest))
            .collect();
        // let mut new_record_file = fs::File::create(record_path)?;
        let mut new_record_file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(record_path)?;
        if FILE_LOCK {
            new_record_file.lock_exclusive().unwrap();
        }
        writeln!(new_record_file, "{}", header)?; // Write the header back
        for line in lines_to_write {
            writeln!(new_record_file, "{}", line)?;
        }
        Ok(())
    }

    pub fn log_exhumed_graves(&self, graves_to_exhume: &[PathBuf]) -> Result<(), Error> {
        // Reopen the record and then delete lines corresponding to exhumed graves
        let record_file = self.open()?;
        self.delete_lines(record_file, graves_to_exhume)
            .map_err(|e| {
                Error::new(
                    e.kind(),
                    format!("Failed to remove unburied files from record: {}", e),
                )
            })
    }

    /// Takes a vector of grave paths and returns the respective lines in the record
    pub fn lines_of_graves<'a>(
        &'a self,
        graves: &'a [PathBuf],
    ) -> impl Iterator<Item = String> + 'a {
        let record_file = self.open().unwrap();
        let mut reader = BufReader::new(record_file).lines();
        reader.next();
        reader
            .map_while(Result::ok)
            .filter(move |line| graves.iter().any(|y| *y == RecordItem::new(line).dest))
    }

    /// Returns an iterator over all graves in the record that are under gravepath
    pub fn seance<'a>(
        &'a self,
        gravepath: &'a PathBuf,
    ) -> io::Result<impl Iterator<Item = RecordItem> + 'a> {
        let record_file = self.open()?;
        let mut reader = BufReader::new(record_file).lines();
        reader.next();
        Ok(reader
            .map_while(Result::ok)
            .map(|line| RecordItem::new(&line))
            .filter(move |record_item| record_item.dest.starts_with(gravepath)))
    }

    /// Write deletion history to record
    pub fn write_log(&self, source: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
        let (source, dest) = (source.as_ref(), dest.as_ref());

        let already_existed = self.path.exists();

        // TODO: The tiny amount of time between the check and the open
        //       could allow for a race condition. But maybe I'm being overkill.

        let mut record_file = if already_existed {
            fs::OpenOptions::new().append(true).open(&self.path)?
        } else {
            fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&self.path)?
        };

        if FILE_LOCK {
            record_file.lock_exclusive().unwrap();
        }

        if !already_existed {
            writeln!(record_file, "Time\tOriginal\tDestination")?;
        }

        writeln!(
            record_file,
            "{}\t{}\t{}",
            Local::now().to_rfc3339(),
            source.display(),
            dest.display()
        )
        .map_err(|e| {
            Error::new(
                e.kind(),
                format!("Failed to write record at {}", &self.path.display()),
            )
        })?;

        Ok(())
    }
}

impl<const FILE_LOCK: bool> Clone for Record<FILE_LOCK> {
    fn clone(&self) -> Self {
        Record {
            path: self.path.clone(),
        }
    }
}
