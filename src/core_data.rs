use std::cell::RefCell;
use std::rc::Rc;

use channel::Channel;
use config::Config;
use logger::log;
use logger::LogLevel::*;
use net::ConnectionState;
use plugin::{Bot, IrcEvent};
use protocol::Protocol;
use plugin_handler::LoadedPlugin;
use user::User;
use server::Server;

#[derive(Debug)]
pub struct NeroData<P: Protocol> {
    pub state: ConnectionState,
    pub now: u64,
    pub uplink: Option<Rc<RefCell<Server<P>>>>,
    pub channels: Vec<Rc<RefCell<Channel<P>>>>,
    pub servers: Vec<Rc<RefCell<Server<P>>>>,
    pub users: Vec<Rc<RefCell<User<P>>>>,
    pub plugins: Vec<LoadedPlugin>,
    pub bots: Vec<Bot>,
    pub events: Vec<IrcEvent>,
    pub config: Config,
    pub write_buffer: Vec<Vec<u8>>,
}

impl<P: Protocol> NeroData<P> {
    pub fn new(config: Config) -> Self {
        Self {
            state: ConnectionState::Connecting,
            now: 0,
            uplink: None,
            channels: Vec::new(),
            servers: Vec::new(),
            users: Vec::new(),
            plugins: Vec::new(),
            events: Vec::new(),
            config: config,
            bots: Vec::new(),
            write_buffer: Vec::new(),
        }
    }

    pub fn add_to_buffer(&mut self, data: &[u8]) {
        self.write_buffer.push(data.into());
    }

    pub fn load_plugins(&mut self) {
        if let Some(plugins) = self.config.plugins.take() {
            for data in &plugins {
                let dynload = LoadedPlugin::new(data.file.as_str());

                match dynload {
                    Ok(mut plugin) => {

                        if let Some(events) = plugin.register_hooks() {
                            for event in events {
                                log(Debug, "CORE_DATA", format!("Registered hook"));
                                self.events.push(event);
                            }
                        }

                        if let Some(bots) = plugin.register_bots() {
                            for bot in bots {
                                self.bots.push(bot);
                            }
                        }

                        log(Debug, "CORE_DATA", format!("Loaded plugin {}", plugin.name()));
                        self.plugins.push(plugin);

                    }
                    Err(e) => {
                        log(Error, "CORE_DATA", format!("Failed to load {} shared object: {}", data.file, e));
                    }
                }
            }

            self.config.plugins = Some(plugins);
        }
    }

    pub fn fire_hook(&mut self, hook: String, origin: &[u8], argc: usize, argv: Vec<Vec<u8>>) {
        use std::ptr;

        for mut event in &mut self.events {
            if event.name == hook {
                let mut plugin = self.plugins.iter_mut().filter(|x| ptr::eq(&***x, event.plugin_ptr)).next().unwrap();
                let _res = (event.f.0)(&mut **plugin, origin, argc, &argv);
            }
        }
    }
}
