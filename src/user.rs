use std::cell::RefCell;
use std::rc::Rc;

use channel::Channel;
use protocol::Protocol;
use protocol::UserExtDefault;
use server::Server;

#[derive(Debug)]
pub struct User<P: Protocol> {
    pub nick: Vec<u8>,
    pub ident: Vec<u8>,
    pub host: Vec<u8>,
    pub ip: Vec<u8>,
    pub gecos: Vec<u8>,
    pub modes: u64,
    pub account: Vec<u8>,
    pub away_message: Vec<u8>,
    pub channels: Vec<Rc<RefCell<Channel<P>>>>,
    pub uplink: Rc<RefCell<Server<P>>>,
    pub ext: P::UserExt,
}

impl<P: Protocol> User<P> {
    pub fn new(nick: &[u8], ident: &[u8], hostname: &[u8], uplink: Rc<RefCell<Server<P>>>) -> Self {
        Self {
            nick: nick.to_vec().clone(),
            ident: ident.to_vec().clone(),
            host: hostname.to_vec().clone(),
            ip: Vec::new(),
            gecos: Vec::new(),
            modes: 0,
            account: Vec::new(),
            away_message: Vec::new(),
            channels: Vec::new(),
            uplink: uplink.clone(),
            ext: P::UserExt::new(),
        }
    }
}
