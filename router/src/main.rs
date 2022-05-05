#[allow(dead_code, unused)]

mod distance;
mod network;
mod route;
mod routing_table;
mod subnet_mask;
mod router;

use std::io;
use std::io::Read;
use crate::router::Router;

fn main() -> std::io::Result<()> {
    let mut handle = io::stdin();
    let mut buffer = String::new();
    handle.read_to_string(&mut buffer)?;

    let router = Router::from(buffer.as_str());
    println!("{router}");
    Ok(())
}
