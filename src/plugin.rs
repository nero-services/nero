use std::any::TypeId;
use core_data::PluginApi;

use channel::BaseChannel;
use server::BaseServer;
use user::BaseUser;

pub type LoadFunc = fn() -> Result<Box<Plugin>, ()>;
pub type UnloadFunc = fn() -> bool;
pub type HookFunc = Box<FnMut(&mut PluginApi, &mut Plugin, &HookData) -> Result<Option<Vec<Vec<u8>>>, HookError>>;

pub struct HookFuncWrapper(pub HookFunc);
pub const MAGIC: &'static str = "COOKIES";

#[derive(Debug)]
pub enum HookType {
    UserConnected,
    PrivmsgChan,
    PrivmsgBot,
}

#[derive(Debug)]
pub struct HookData {
    pub hook_type: HookType,
    pub channel: Option<BaseChannel>,
    pub user: Option<BaseUser>,
    pub server: Option<BaseServer>,
    pub origin: Vec<u8>,
    pub message: Vec<u8>,
    pub argc: usize,
    pub argv: Vec<Vec<u8>>,
}

impl HookData {
    pub fn new(hook_type: HookType) -> Self {
        Self {
            hook_type: hook_type,
            channel: None,
            user: None,
            server: None,
            origin: Vec::new(),
            message: Vec::new(),
            argc: 0,
            argv: Vec::new(),
        }
    }
}

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
    pub channels: Vec<BotChannel>,
}

#[derive(Debug, Clone)]
pub struct BotChannel {
    pub name: String,
    pub chanmodes: String,
    pub umodes: String,
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
