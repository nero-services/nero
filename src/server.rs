use std::cell::RefCell;
use std::rc::Rc;

use user::User;
use protocol::Protocol;
use protocol::ServExtDefault;

#[derive(Debug)]
pub struct Server<P: Protocol> {
    pub hostname: Vec<u8>,
    pub description: Vec<u8>,
    pub uplink: Option<Rc<RefCell<Server<P>>>>,
    pub children: Vec<Rc<RefCell<Server<P>>>>,
    pub users: Vec<Rc<RefCell<User<P>>>>,
    pub hops: i8,
    pub boot: u64,
    pub link_time: u64,
    pub ext: P::ServExt,
}

impl<P: Protocol> Server<P> {
    pub fn new(hostname: &[u8], description: &[u8]) -> Self {
        Self {
            hostname: hostname.to_vec().clone(),
            description: description.to_vec().clone(),
            uplink: None,
            children: Vec::new(),
            users: Vec::new(),
            hops: 0,
            boot: 0,
            link_time: 0,
            ext: P::ServExt::new(),
        }
    }
}
