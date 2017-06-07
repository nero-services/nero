use toml;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub uplink: Uplink,
    pub plugins: Option<Vec<Plugin>>,
}

#[derive(Debug, Deserialize)]
pub struct Uplink {
    pub ip: String,
    pub port: i32,
    pub protocol: String,
    pub hostname: String,
    pub description: String,
    pub send_pass: String,
    pub recv_pass: String,
    pub numeric: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Plugin {
    pub file: String,
    pub load: Option<bool>,
}

pub fn get_protocol() -> Result<String, Box<::std::error::Error>> {
    let file = File::open("etc/nero.toml")?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();

    buf_reader.read_to_string(&mut contents)?;

    let cfg: Config = toml::from_str(&contents)?;

    Ok(cfg.uplink.protocol)
}

pub fn load() -> Result<Result<Config, toml::de::Error>, ::std::io::Error> {
    let file = File::open("etc/nero.toml")?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;

    Ok(toml::from_str(&contents))
}
