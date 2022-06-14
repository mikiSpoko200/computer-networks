//! Mikołaj Depta 328690
//!
//! Utility module that contains convenience wrapper functions.

#![allow(dead_code)]

use std::process;

pub fn fail_with_message(message: &str) -> ! {
    eprintln!("{}", message);
    process::exit(1)
}

pub trait FailWithMessage<T> {
    fn or_fail_with_message(self, message: &str) -> T;
}

impl<T, E> FailWithMessage<T> for Result<T, E> {
    fn or_fail_with_message(self, message: &str) -> T {
        match self {
            Ok(val) => val,
            Err(_) => fail_with_message(message),
        }
    }
}

impl<T> FailWithMessage<T> for Option<T> {
    fn or_fail_with_message(self, message: &str) -> T {
        match self {
            Some(val) => val,
            None => fail_with_message(message),
        }
    }
}
