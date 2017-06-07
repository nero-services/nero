use core_data::NeroData;

pub trait Protocol: Sized + Send + Sync + 'static {
    type ChanExt: ChanExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    type UserExt: UserExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    type ServExt: ServExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    type MemberExt: MemberExtDefault + Send + Sync + ::std::fmt::Debug + 'static;
    // type LoggerExt: LoggerExtDefault + Send + Sync + ::std::fmt::Debug + 'static;

    fn new() -> Self;
    fn start_handshake(&mut self, me: &mut NeroData<Self>, ret: &mut Vec<Vec<u8>>);
    fn process(&self, message: &[u8], me: &mut NeroData<Self>, ret: &mut Vec<Vec<u8>>);
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
