use std::cell::RefCell;
use std::rc::Rc;

use user::User;
use protocol::Protocol;
use protocol::ServExtDefault;

#[derive(Debug)]
pub struct BaseServer {
    pub hostname: Vec<u8>,
    pub description: Vec<u8>,
    pub hops: i8,
    pub boot: u64,
    pub link_time: u64,
}

#[derive(Debug)]
pub struct Server<P: Protocol> {
    pub base: BaseServer,
    pub uplink: Option<Rc<RefCell<Server<P>>>>,
    pub children: Vec<Rc<RefCell<Server<P>>>>,
    pub users: Vec<Rc<RefCell<User<P>>>>,
    pub ext: P::ServExt,
}

impl BaseServer {
    pub fn new(hostname: &[u8], description: &[u8]) -> Self {
        Self {
            hostname: hostname.to_vec().clone(),
            description: description.to_vec().clone(),
            hops: 0,
            boot: 0,
            link_time: 0,
        }
    }
}

impl<P: Protocol> Server<P> {
    pub fn new(hostname: &[u8], description: &[u8]) -> Self {
        let base = BaseServer::new(hostname, description);
        Self {
            base: base,
            uplink: None,
            children: Vec::new(),
            users: Vec::new(),
            ext: P::ServExt::new(),
        }
    }
}
