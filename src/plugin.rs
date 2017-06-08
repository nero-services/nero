use std::any::TypeId;
use core_data::PluginApi;

pub type LoadFunc = fn() -> Result<Box<Plugin>, ()>;
pub type UnloadFunc = fn() -> bool;
pub type HookFunc = Box<FnMut(&mut PluginApi, &mut Plugin, &[u8], usize, &[Vec<u8>]) -> Result<Option<Vec<Vec<u8>>>, HookError>>;

pub struct HookFuncWrapper(pub HookFunc);
pub const MAGIC: &'static str = "PANCAKES";

#[derive(Debug)]
pub struct HookError {
    pub message: String,
}

#[derive(Debug)]
pub struct Bot {
    pub nick: String,
    pub ident: String,
    pub hostname: String,
    pub gecos: String,
}

impl ::std::fmt::Debug for HookFuncWrapper {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "HookFunc")
    }
}

#[derive(Debug)]
pub struct IrcEvent {
    pub plugin_ptr: *const Plugin,
    pub name: String,
    pub f: HookFuncWrapper,
}

pub trait Plugin: 'static {
    fn name(&mut self) -> String;
    fn description(&mut self) -> String;
    fn register_hooks(&mut self) -> Option<Vec<IrcEvent>>;
    unsafe fn get_type_id(&self) -> TypeId { TypeId::of::<Self>() }
    fn register_bots(&mut self) -> Option<Vec<Bot>>;
    // fn bot_privmsg(&mut self, target: &[u8], message: &[u8]);
}

impl Plugin {
    pub fn downcast_mut<T: Plugin>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            unsafe {
                Some(&mut *(self as *mut Plugin as *mut T))
            }
        } else {
            None
        }
    }

    #[inline]
    pub fn is<T: Plugin>(&self) -> bool {
        let t = TypeId::of::<T>();
        let boxed = unsafe { self.get_type_id() };
        t == boxed
    }
}
