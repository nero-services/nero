use std::cell::RefCell;
use std::rc::Rc;

use channel::Channel;
use config::Config;
use logger::log;
use logger::LogLevel::*;
use net::ConnectionState;
use plugin::IrcEvent;
use protocol::Protocol;
use plugin::{PluginApi, HookData};
use plugin_handler::LoadedPlugin;
use user::{BaseUser, User};
use server::Server;

pub trait Target {
    fn get_target(&self) -> Vec<u8>;
}

impl<P: Protocol> PluginApi for NeroData<P> {
    fn get_user_by_nick(&self, nick: &[u8]) -> Option<BaseUser> {
        for user in &self.users {
            let borrowed_user = user.borrow();
            if borrowed_user.base.nick == nick.to_vec() {
                return Some(borrowed_user.base.clone());
            }
        }

        None
    }

    fn get_user_by_numeric(&self, nick: &[u8]) -> Option<BaseUser> {
        let proto = &self.protocol;
        proto.find_user_by_numeric(&self.users, nick)
    }

    fn get_user_count(&self) -> usize {
        self.users.len()
    }

    fn get_channel_count(&self) -> usize {
        self.channels.len()
    }

    fn get_server_count(&self) -> usize {
        self.servers.len()
    }

    fn send_notice(&mut self, source: &BaseUser, target: &Target, message: &[u8]) {
        self.send_textmessage(source, target, message, false);
    }

    fn send_privmsg(&mut self, source: &BaseUser, target: &Target, message: &[u8]) {
        self.send_textmessage(source, target, message, true);
    }

    fn send_textmessage(&mut self, source: &BaseUser, target: &Target, message: &[u8], privmsg: bool) {
        let target_name = target.get_target();
        let proto = &self.protocol;
        let users = &self.users;

        if privmsg {
            proto.send_privmsg(users, &mut self.write_buffer, &source, &target_name, message);
        } else {
            proto.send_notice(users, &mut self.write_buffer, &source, &target_name, message);
        }
    }

    fn send_privmsg_raw_target(&mut self, source: &BaseUser, target: &[u8], message: &[u8]) {
        let proto = &self.protocol;
        let users = &self.users;
        proto.send_privmsg(users, &mut self.write_buffer, &source, target, message);
    }
}

#[derive(Debug)]
pub struct NeroData<P: Protocol> {
    pub state: ConnectionState,
    pub now: u64,
    pub uplink: Option<Rc<RefCell<Server<P>>>>,
    pub me: Rc<RefCell<Server<P>>>,
    pub channels: Vec<Rc<RefCell<Channel<P>>>>,
    pub unbursted_channels: Vec<Vec<u8>>,
    pub servers: Vec<Rc<RefCell<Server<P>>>>,
    pub users: Vec<Rc<RefCell<User<P>>>>,
    pub plugins: Vec<LoadedPlugin>,
    pub events: Vec<IrcEvent>,
    pub config: Config,
    pub write_buffer: Vec<Vec<u8>>,
    pub protocol: P,
}

impl<P: Protocol> NeroData<P> {
    pub fn new(config: Config) -> Self {
        let my_hostname = config.uplink.hostname.clone().into_bytes();
        let my_description = config.uplink.description.clone().into_bytes();
        let me = Rc::new(RefCell::new(Server::<P>::new(&my_hostname, &my_description)));

        let mut s = Self {
            state: ConnectionState::Connecting,
            now: 0,
            uplink: None,
            me: me.clone(),
            channels: Vec::new(),
            unbursted_channels: Vec::new(),
            servers: Vec::new(),
            users: Vec::new(),
            plugins: Vec::new(),
            events: Vec::new(),
            config: config,
            write_buffer: Vec::new(),
            protocol: P::new(),
        };

        s.servers.push(me);
        s
    }

    pub fn add_to_buffer(&mut self, data: &[u8]) {
        self.write_buffer.push(data.into());
    }

    pub fn setup(&mut self) {
        let config = &self.config;
        let mut me_borrow = self.me.borrow_mut();
        self.protocol.setup(&mut me_borrow, config);
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
                                let protocol = ::std::mem::replace(&mut self.protocol, P::new());
                                protocol.add_local_bot(self, &bot);
                                self.protocol = protocol;
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

    pub fn fire_hook(&mut self, hook_data: &HookData) {
        use std::ptr;
        use std::mem;

        let mut events = mem::replace(&mut self.events, Vec::new());
        let mut plugins = mem::replace(&mut self.plugins, Vec::new());

        for event in &mut events {
            if event.event_type == hook_data.hook_type {
                let plugin = plugins.iter_mut().filter(|x| ptr::eq(&***x, event.plugin_ptr)).next().unwrap();
                match (event.f.0)(self, &mut **plugin, hook_data) {
                    Ok(_) => {},
                    Err(e) => {
                        log(Error, "PLUGIN", format!("Error from plugin: {}", e.message));
                    }
                }
            }
        }

        self.events = events;
        self.plugins = plugins;
    }
}
