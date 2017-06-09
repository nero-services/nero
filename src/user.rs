use std::cell::RefCell;
use std::rc::Rc;

use channel::Channel;
use core_data::Target;
use protocol::Protocol;
use protocol::UserExtDefault;
use server::Server;

#[derive(Debug, Clone)]
pub struct BaseUser {
    pub nick: Vec<u8>,
    pub ident: Vec<u8>,
    pub host: Vec<u8>,
    pub ip: Vec<u8>,
    pub gecos: Vec<u8>,
    pub modes: u64,
    pub account: Vec<u8>,
    pub away_message: Vec<u8>,
}

#[derive(Debug)]
pub struct User<P: Protocol> {
    pub base: BaseUser,
    pub channels: Vec<Rc<RefCell<Channel<P>>>>,
    pub uplink: Rc<RefCell<Server<P>>>,
    pub ext: P::UserExt,
}

impl BaseUser {
    pub fn new(nick: &[u8], ident: &[u8], hostname: &[u8]) -> Self {
        Self {
            nick: nick.to_vec().clone(),
            ident: ident.to_vec().clone(),
            host: hostname.to_vec().clone(),
            ip: Vec::new(),
            gecos: Vec::new(),
            modes: 0,
            account: Vec::new(),
            away_message: Vec::new(),
        }
    }
}

impl Target for BaseUser {
    fn get_target(&self) -> Vec<u8> {
        return self.nick.to_vec().clone();
    }
}

impl<P: Protocol> User<P> {
    pub fn new(nick: &[u8], ident: &[u8], hostname: &[u8], uplink: Rc<RefCell<Server<P>>>) -> Self {
        let base = BaseUser::new(nick, ident, hostname);
        Self {
            base: base,
            channels: Vec::new(),
            uplink: uplink.clone(),
            ext: P::UserExt::new(),
        }
    }
}
