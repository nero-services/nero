use std::cell::RefCell;
use std::rc::Rc;

use protocol::{Protocol, MemberExtDefault};
use user::User;

#[derive(Debug)]
pub struct BaseChannelMember {
    pub modes: u64,
    pub idle: u64,
}

#[derive(Debug)]
pub struct ChannelMember<P: Protocol> {
    pub base: BaseChannelMember,
    pub user: Rc<RefCell<User<P>>>,
    pub ext: P::MemberExt,
}

impl BaseChannelMember {
    pub fn new() -> Self {
        Self {
            modes: 0,
            idle: 0,
        }
    }
}

impl<P> ChannelMember<P> where P: Protocol {
    pub fn new(user: Rc<RefCell<User<P>>>) -> Self {
        let base = BaseChannelMember::new();
        Self {
            base: base,
            user: user.clone(),
            ext: P::MemberExt::new(),
        }
    }
}
