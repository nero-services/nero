extern crate base64;
extern crate libloading;
#[macro_use]
extern crate bitflags;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate tokio_io;
extern crate toml;

use tokio_core::reactor::Core;
use p10::P10;

pub mod channel;
pub mod channel_member;
pub mod core_data;
pub mod config;
pub mod logger;
pub mod net;
pub mod p10;
pub mod plugin;
pub mod protocol;
pub mod server;
pub mod user;
pub mod utils;
pub mod plugin_handler;

pub fn run() {
    let mut core = Core::new().unwrap();

    let connection = match config::get_protocol() {
        Ok(p) => {
            match &p as &str {
                "P10" => net::boot::<P10>(core.handle()),
                _ => {
                    println!("Only P10 is currently supported");
                    return;
                }
            }
        },
        Err(e) => {
            println!("Failed to read protocol from config: {}", e);
            return;
        }
    };

    core.run(connection).unwrap();
}
