#[derive(Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

pub fn log(level: LogLevel, module: &'static str, message: String) {
    let prefix = match level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
        LogLevel::Fatal => "fatal",
    };

    println!("L: ({}/{}): {}", prefix, module, message);
}
