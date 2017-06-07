use libloading;

use logger::log;
use logger::LogLevel::*;
use plugin::Plugin;

#[derive(Debug)]
pub struct LoadedPlugin {
    lib: libloading::Library,
    plugin: Plugin
}

#[derive(Debug)]
pub struct IrcEvent {
    name: String,
    origin: Vec<u8>,
    argc: usize,
    argv: Vec<Vec<u8>>,
}

impl LoadedPlugin {
    pub fn new(name: &str) -> Result<Self, ::std::io::Error> {
        let lib = libloading::Library::new(name)?;

        let plugin = unsafe {
            let initialize_plugin: libloading::Symbol<fn() -> Plugin> = lib.get(b"nero_initialize")?;
            initialize_plugin()
        };

        Ok(Self {
            lib,
            plugin,
        })
    }
}

impl ::std::ops::Deref for LoadedPlugin {
    type Target = Plugin;

    fn deref(&self) -> &Self::Target {
        &self.plugin
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        let result = (self.plugin.unload)();
        if ! result {
            log(Error, "PLUGIN", format!("Failure when unloading plugin {}", self.plugin.name));
        }
    }
}
