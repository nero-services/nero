use std::cell::RefCell;
use std::rc::Rc;

use protocol::{Protocol, MemberExtDefault};
use user::User;

#[derive(Debug)]
pub struct ChannelMember<P: Protocol> {
    pub user: Rc<RefCell<User<P>>>,
    pub modes: u64,
    pub idle: u64,
    pub ext: P::MemberExt,
}

impl<P> ChannelMember<P> where P: Protocol {
    pub fn new(user: Rc<RefCell<User<P>>>) -> Self {
        Self {
            user: user.clone(),
            modes: 0,
            idle: 0,
            ext: P::MemberExt::new(),
        }
    }
}
