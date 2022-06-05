use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Stdout, Write};
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

fn main() {
    println!("Hello, world!");
}

/// Type of http method
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum RequestMethod {
    GET,
    POST,
}


// region Loggers
pub trait Logger {
    fn log<T: AsRef<str>>(&mut self, message: &T);

    fn format<T: AsRef<str>>(&self, message: &T) -> String {
        use std::time::Instant;

        let now = Instant::now();
        format!("[{now}] {message}")
    }
}

struct StdoutLogger(Stdout);

impl StdoutLogger {
    pub fn new() -> Self {
        Self(io::stdout())
    }
}

impl Logger for StdoutLogger {
    fn log<T: AsRef<str>>(&mut self, message: &T) {
        self.0.write_all(self.format(message).as_bytes()).expect("write to stdout failed");
    }
}

struct FileLogger(File);

impl FileLogger {
    pub fn new(log_file_path: &Path) -> Self {
        let handle = OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_file_path)
            .expect("log file creation failed");
        Self(handle)
    }
}

impl Logger for StdoutLogger {
    fn log<T: AsRef<str>>(&mut self, message: &T) {
        self.0.write_all(self.format(message).as_bytes()).expect("write to log file failed");
    }
}
// endregion


struct ParseHttpHeaderError {
    header_
}

impl FromStr for RequestMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

pub enum RequestHeader {
    Host,
    Connection
}

pub enum ResponseHeader {

}

struct HttpRequest {
    method: RequestMethod,
    headers: Vec<>
}

