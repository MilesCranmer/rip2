use chrono::Local;
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::util;

const RECORD: &str = ".record";

pub struct RecordItem<'a> {
    _time: &'a str,
    pub orig: &'a Path,
    pub dest: &'a Path,
}

impl RecordItem<'_> {
    /// Parse a line in the record into a `RecordItem`
    pub fn new(line: &str) -> RecordItem {
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
}

pub struct Record {
    path: PathBuf,
}

impl Record {
    pub fn new(graveyard: &Path) -> Record {
        Record {
            path: graveyard.join(RECORD),
        }
    }

    pub fn open(&self) -> Result<fs::File, Error> {
        fs::File::open(&self.path)
            .map_err(|_| Error::new(ErrorKind::NotFound, "Failed to read record!"))
    }

    /// Return the path in the graveyard of the last file to be buried.
    /// As a side effect, any valid last files that are found in the record but
    /// not on the filesystem are removed from the record.
    pub fn get_last_bury(&self) -> Result<PathBuf, Error> {
        // record: impl AsRef<Path>
        let record_file = self.open()?;
        let contents = {
            let path_f = PathBuf::from(&self.path);
            fs::read_to_string(path_f)?
        };

        // This will be None if there is nothing, or Some
        // if there is items in the vector
        let mut graves_to_exhume = Vec::new();
        for entry in contents.lines().rev().map(RecordItem::new) {
            // Check that the file is still in the graveyard.
            // If it is, return the corresponding line.
            if util::symlink_exists(entry.dest) {
                if !graves_to_exhume.is_empty() {
                    self.delete_lines(record_file, &graves_to_exhume)?;
                }
                return Ok(PathBuf::from(entry.dest));
            } else {
                // File is gone, mark the grave to be removed from the record
                graves_to_exhume.push(PathBuf::from(entry.dest));
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
        // the ones matching the exhumed graves.  Store them in a vector
        // since we'll be overwriting the record in-place.
        let lines_to_write: Vec<String> = BufReader::new(record_file)
            .lines()
            .map_while(Result::ok)
            .filter(|line| !graves.iter().any(|y| y == RecordItem::new(line).dest))
            .collect();
        let mut mutable_record_file = fs::File::create(record_path)?;
        for line in lines_to_write {
            writeln!(mutable_record_file, "{}", line)?;
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
        BufReader::new(record_file)
            .lines()
            .map_while(Result::ok)
            .filter(move |line| graves.iter().any(|y| y == RecordItem::new(line).dest))
    }

    /// Returns an iterator over all graves in the record that are under gravepath
    pub fn seance<T: AsRef<str>>(&self, gravepath: T) -> impl Iterator<Item = PathBuf> {
        let record_file = self.open().unwrap();
        BufReader::new(record_file)
            .lines()
            .map_while(Result::ok)
            .map(|line| PathBuf::from(RecordItem::new(&line).dest))
            .filter(move |d| d.starts_with(gravepath.as_ref()))
    }

    /// Write deletion history to record
    pub fn write_log(&self, source: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
        let (source, dest) = (source.as_ref(), dest.as_ref());
        let mut record_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
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
