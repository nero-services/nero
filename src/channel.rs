use std::cell::RefCell;
use std::rc::Rc;

use channel_member::ChannelMember;
use protocol::{Protocol, ChanExtDefault};

#[derive(Debug)]
pub struct Channel<P: Protocol> {
    pub name: Vec<u8>,
    pub topic: Vec<u8>,
    pub topic_nick: Vec<u8>,
    pub topic_time: u64,
    pub created: u64,
    pub modes: u64,
    pub limit: u64,
    pub key: Option<Vec<u8>>,
    pub bans: Vec<Vec<u8>>,
    pub members: Vec<Rc<RefCell<ChannelMember<P>>>>,
    pub ext: P::ChanExt,
}

impl<P> Channel<P> where P: Protocol {
    pub fn new(name: &[u8], created: u64) -> Self {
        Self {
            name: name.to_vec().clone(),
            topic: Vec::new(),
            topic_nick: Vec::new(),
            topic_time: 0,
            created: created,
            modes: 0,
            limit: 0,
            key: None,
            bans: Vec::new(),
            members: Vec::new(),
            ext: P::ChanExt::new(),
        }
    }
}
