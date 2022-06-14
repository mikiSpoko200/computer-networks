//! Mikołaj Depta 328690
//!
//! Logging utilities.

use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufWriter, Stdout, prelude::*};
use std::path::Path;

pub trait Logger {
    fn log<T: AsRef<str>>(&mut self, message: &T);

    fn format<T: AsRef<str>>(&self, message: &T) -> String {
        use std::time::Instant;

        let now = Instant::now();
        format!("[{:?}] {}", now, message.as_ref())
    }
}

struct StdoutLogger(BufWriter<Stdout>);

impl StdoutLogger {
    pub fn new() -> Self {
        Self(BufWriter::new(io::stdout()))
    }
}

impl Logger for StdoutLogger {
    fn log<T: AsRef<str>>(&mut self, message: &T) {
        self.0
            .write_all(self.format(message).as_bytes())
            .expect("write to stdout failed");
    }
}

struct FileLogger(BufWriter<File>);

impl FileLogger {
    pub fn new(log_file_path: &Path) -> Self {
        let handle = OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_file_path)
            .expect("log file creation failed");
        Self(BufWriter::new(handle))
    }
}

impl Logger for FileLogger {
    fn log<T: AsRef<str>>(&mut self, message: &T) {
        self.0
            .write_all(self.format(message).as_bytes())
            .expect("write to log file failed");
    }
}
