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

/* Note: Issues with recv_from, server is detected but no bytes are being sent in response.
    maybe newline character does not match specification?
*/

fn main() {
    let config = DownloaderConfig::try_from(env::args());
    let mut downloader = Downloader::from(config);
    downloader.download()
}
