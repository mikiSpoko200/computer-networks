//! Utility module that contains convenience wrapper functions.

#![allow(dead_code)]

use std::process;

pub fn fail_with_message(message: impl AsRef<str>) -> ! {
    eprintln!("{}", message.as_ref());
    process::exit(1)
}
