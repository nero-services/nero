use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use config::Config;
use core_data::NeroData;
use plugin::Bot;
use server::Server;
use user::{User, BaseUser};

pub trait Protocol: Sized + Send + Sync + 'static {
    type ChanExt: ChanExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    type UserExt: UserExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    type ServExt: ServExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    type MemberExt: MemberExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    // type LoggerExt: LoggerExtDefault + Send + Sync + ::std::fmt::Debug + 'static;

    fn new() -> Self;
    fn setup(&self, me: &mut RefMut<Server<Self>>, core_data: &Config);
    fn start_handshake(&mut self, me: &mut NeroData<Self>);
    fn process(&self, message: &[u8], me: &mut NeroData<Self>);
    fn find_user_by_numeric(&self, users: &Vec<Rc<RefCell<User<Self>>>>, numeric: &[u8]) -> Option<BaseUser>;
    fn send_privmsg(&self, users: &Vec<Rc<RefCell<User<Self>>>>, write_buffer: &mut Vec<Vec<u8>>, source: &BaseUser, target: &[u8], message: &[u8]);
    fn send_notice(&self, users: &Vec<Rc<RefCell<User<Self>>>>, write_buffer: &mut Vec<Vec<u8>>, source: &BaseUser, target: &[u8], message: &[u8]);
    fn add_local_bot(&self, core_data: &mut NeroData<Self>, bot: &Bot);
}

pub trait ChanExtDefault {
    fn new() -> Self;
}

pub trait UserExtDefault {
    fn new() -> Self;
}

pub trait ServExtDefault {
    fn new() -> Self;
}

pub trait MemberExtDefault {
    fn new() -> Self;
}

// pub trait LoggerExtDefault {
//     fn new() -> Self;
// }
