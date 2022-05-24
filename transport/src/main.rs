#![allow(dead_code)]

mod registry;
mod segment;
mod util;
mod messages;
mod window;
mod downloader;

use libc;
use std::env;
use downloader::Downloader;
use crate::downloader::DownloaderConfig;

fn main() {
    let config = DownloaderConfig::try_from(env::args());
    let mut downloader = Downloader::from(config);
    downloader.download()
}
