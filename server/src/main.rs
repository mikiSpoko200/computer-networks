//! Mikołaj Depta 328690
#![allow(dead_code)]

mod http;
mod logger;
mod resources;
mod util;
mod server;
mod registry;

use libc;

/* Resources:
Max accepted size of GET request: https://stackoverflow.com/questions/2659952/maximum-length-of-http-get-request
RFC HTTP 1.1: https://datatracker.ietf.org/doc/html/rfc2616
RFC TCP: https://datatracker.ietf.org/doc/html/rfc793
*/


fn main() {
    println!("Hello, world!");
}
