use libloading;

use logger::log;
use logger::LogLevel::*;
use plugin::{Plugin, LoadFunc, MAGIC};

pub struct LoadedPlugin {
    _lib: libloading::Library,
    plugin: Box<Plugin>
}

impl LoadedPlugin {
    pub fn new(name: &str) -> Result<Self, ::std::io::Error> {
        let lib = libloading::Library::new(name)?;

        let magic = unsafe {
            let magic_symbol: libloading::Symbol<&'static &'static str> = lib.get(b"PLUGIN_MAGIC")?;
            **magic_symbol
        };

        if magic != MAGIC {
            return Err(::std::io::Error::new(::std::io::ErrorKind::Other,
                format!("Invalid magic number, expected {} but got {}", MAGIC, magic)));
        }

        let plugin = unsafe {
            let initialize_plugin: libloading::Symbol<LoadFunc> = lib.get(b"nero_initialize")?;
            initialize_plugin().map_err(|_| {
                log(Error, "plugin_handler", format!("Failed to read plugin initializer"));
                ::std::io::Error::new(::std::io::ErrorKind::Other, format!("Failed to read symbols"))
            })?
        };

        Ok(Self {
            _lib: lib,
            plugin,
        })
    }
}

impl ::std::ops::Deref for LoadedPlugin {
    type Target = Plugin;

    fn deref(&self) -> &Self::Target {
        &*self.plugin
    }
}

impl ::std::ops::DerefMut for LoadedPlugin {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.plugin
    }
}

impl ::std::fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "LoadedPlugin")
    }
}
