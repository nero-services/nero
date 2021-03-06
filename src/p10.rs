use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use core_data::{NeroData, Target};
use net::ConnectionState;

use channel::Channel;
use channel_member::ChannelMember;
use config::Config;
use logger::log;
use logger::LogLevel::*;
use plugin::Bot;
use protocol::{Protocol, ChanExtDefault, MemberExtDefault, ServExtDefault, UserExtDefault};
use user::{BaseUser, User};
use utils::{epoch_int, dv, split_string, unsplit_string, u8_slice_to_lower, ceiling_division, inttobase64};
use server::Server;

#[derive(Debug, Copy, Clone)]
pub struct P10 {
    skew: u64,
}

// Custom P10 struct extensions

#[derive(Debug)]
pub struct P10ChannelExt {
    pub delayed_join: bool,
    pub upass: Option<Vec<u8>>,
    pub apass: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct P10MemberExt {
    pub oplevel: u64,
}

#[derive(Debug)]
pub struct P10UserExt {
    pub numeric: Vec<u8>,
    pub fakeident: Vec<u8>,
    pub fakehost: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct Gline {
    pub issued: u64,
    pub lastmod: u64,
    pub expires: u64,
    pub lifetime: u64,
    pub issuer: Vec<u8>,
    pub target: Vec<u8>,
    pub reason: Vec<u8>,
}

#[derive(Debug)]
pub struct P10ServExt {
    pub numeric: Vec<u8>,
    pub glines: Vec<Gline>,
    pub self_burst: bool,
    pub numeric_accum: u64,
}

impl Gline {
    pub fn new(target: &[u8]) -> Self {
        Self {
            issued: 0,
            lastmod: 0,
            expires: 0,
            lifetime: 0,
            issuer: Vec::new(),
            target: target.to_vec(),
            reason: Vec::new(),
        }
    }
}

// IRCu/P10 modes
bitflags! {
    pub struct P10UserModes: u64 {
        const UMODE_OPER         = 1 << 0;
        const UMODE_INVISIBLE    = 1 << 1;
        const UMODE_WALLOP       = 1 << 2;
        const UMODE_DEAF         = 1 << 3;
        const UMODE_SERVICE      = 1 << 4;
        const UMODE_GLOBAL       = 1 << 5;
        const UMODE_NOCHAN       = 1 << 6;
        const UMODE_NOIDLE       = 1 << 7;
        const UMODE_HIDDEN_HOST  = 1 << 8;
        const UMODE_STAMPED      = 1 << 9;
    }
}

bitflags! {
    pub struct P10ChannelModes: u64 {
        const CMODE_PRIVATE     = 1 << 0;
        const CMODE_SECRET      = 1 << 1;
        const CMODE_MODERATED   = 1 << 2;
        const CMODE_TOPICLIMIT  = 1 << 3;
        const CMODE_INVITEONLY  = 1 << 4;
        const CMODE_NOPRIVMSGS  = 1 << 5;
        const CMODE_KEY         = 1 << 6;
        const CMODE_BAN         = 1 << 7;
        const CMODE_LIMIT       = 1 << 8;
        const CMODE_DELAYJOINS  = 1 << 9;
        const CMODE_REGONLY     = 1 << 10;
        const CMODE_NOCOLORS    = 1 << 11;
        const CMODE_NOCTCPS     = 1 << 12;
        const CMODE_REGISTERED  = 1 << 13;
        const CMODE_APASS       = 1 << 14;
        const CMODE_UPASS       = 1 << 15;
    }
}

bitflags! {
    pub struct P10MemberModes: u64 {
        const MMODE_CHANOP      = 1 << 0;
        const MMODE_VOICE       = 1 << 1;
        const MMODE_HIDDEN      = 1 << 2;
    }
}

impl ServExtDefault for P10ServExt {
    fn new() -> Self {
        Self {
            numeric: Vec::new(),
            glines: Vec::new(),
            self_burst: true,
            numeric_accum: 0,
        }
    }
}

impl UserExtDefault for P10UserExt {
    fn new() -> Self {
        Self {
            numeric: Vec::new(),
            fakeident: Vec::new(),
            fakehost: Vec::new(),
            timestamp: 0,
        }
    }
}

impl Target for P10UserExt {
    fn get_target(&self) -> Vec<u8> {
        return self.numeric.to_vec().clone();
    }
}

impl ChanExtDefault for P10ChannelExt {
    fn new() -> Self {
        Self {
            delayed_join: false,
            upass: None,
            apass: None,
        }
    }
}

impl MemberExtDefault for P10MemberExt {
    fn new() -> Self {
        Self {
            oplevel: 0,
        }
    }
}

impl Protocol for P10 {
    type ChanExt = P10ChannelExt;
    type UserExt = P10UserExt;
    type ServExt = P10ServExt;
    type MemberExt = P10MemberExt;

    fn new() -> Self {
        Self {
            skew: 0,
        }
    }

    fn setup(&self, me: &mut RefMut<Server<Self>>, config: &Config) {
        if me.ext.numeric.len() == 0 {
            me.ext.numeric = config.uplink.numeric.clone().unwrap().into_bytes();
        }
    }

    fn start_handshake(&mut self, core_data: &mut NeroData<Self>) {
        if core_data.state == ConnectionState::Connecting {
            core_data.state = ConnectionState::Bursting;

            let send_pass = &core_data.config.uplink.send_pass.clone();
            let hostname = &core_data.config.uplink.hostname.clone();
            let description = &core_data.config.uplink.description.clone();
            let numeric_optional = core_data.config.uplink.numeric.clone();
            let numeric = &numeric_optional.unwrap();
            let epoch = epoch_int();

            core_data.add_to_buffer(&format!("PASS :{}", send_pass).as_bytes());
            core_data.add_to_buffer(&format!("SERVER {} 1 {} {} J10 {}A]] +s6 :{}", hostname, epoch, epoch, numeric, description).as_bytes());
        }
    }

    fn process(&self, message: &[u8], core_data: &mut NeroData<Self>) {
        core_data.now = epoch_int() + self.skew;

        let (argc, argv): (usize, Vec<Vec<u8>>) = split_line(message, true, 200);
        // println!("argc={}, argv={:#?}", argc, argv.iter().map(|x| -> String {String::from_utf8_lossy(x).into_owned()}).collect::<Vec<_>>());

        // Did not get data from uplink
        if argv.len() == 0 {
            return;
        }

        let cmd: usize = if argv[0].len() < 2 || argv[0].len() < 3 || core_data.uplink.is_some() {
            1
        } else {
            0
        };

        if &argv[0] != b"SERVER" && &argv[0] != b"PASS" {
            assert!(core_data.uplink.is_some());
            assert_eq!(cmd, 1);
        }

        let mut origin: Vec<u8> = Vec::new();

        if argc > cmd {
            if cmd > 0 {
                if argv[0][0] == b':' {
                    origin = argv[0][..argv[0].len()-1].to_vec();
                } else if argv[0].len() < 2 || argv[0].len() < 3 {
                    // println!("Looking for server with numeric {}", dv(&argv[0]));
                    match find_server_numeric(core_data, &argv[0].to_vec()) {
                        Some(ref server) => {
                            origin = server.borrow().ext.numeric.clone();
                        }
                        None => {},
                    }
                } else {
                    // println!("Looking for nick with numeric {}", dv(&argv[0]));
                    match find_user_numeric(core_data, &argv[0].to_vec()) {
                        Some(ref user) => {
                            origin = user.borrow().ext.numeric.clone();
                        }
                        None => {},
                    }
                }
            }

            let command: &[u8] = &argv[cmd];
            let mut newargv: Vec<Vec<u8>> = argv.clone();
            if cmd > 0 {
                newargv = argv[1..].to_vec();
            }

            let result = match command {
                b"SERVER" => p10_cmd_server(core_data, &origin, argc-cmd, &newargv),
                b"PASS" => p10_cmd_pass(core_data, &origin, argc-cmd, &newargv),
                b"S" => p10_cmd_server(core_data, &origin, argc-cmd, &newargv),
                b"N" => p10_cmd_n(core_data, &origin, argc-cmd, &newargv),
                b"Q" => p10_cmd_q(core_data, &origin, argc-cmd, &newargv),
                b"B" => p10_cmd_b(core_data, argc-cmd, &newargv),
                b"T" => p10_cmd_t(core_data, &origin, argc-cmd, &newargv),
                b"G" => p10_cmd_g(core_data, &origin, argc-cmd, &newargv),
                b"P" => p10_cmd_textmessage(core_data, &origin, argc-cmd, &newargv, true),
                b"O" => p10_cmd_textmessage(core_data, &origin, argc-cmd, &newargv, false),
                b"GL" => p10_cmd_gl(core_data, &origin, argc-cmd, &newargv),
                b"EB" => p10_cmd_eb(core_data, &origin),
                b"EA" => p10_cmd_ea(core_data, &origin),
                _ => Err(()),
            };

            // println!("Looking for command '{}'", dv(&command));

            if let Err(_) = result {
                log(Error, "MAIN", format!("PARSE ERROR: {}", dv(&message)));
            }
        }
    }

    fn find_user_by_numeric(&self, users: &Vec<Rc<RefCell<User<P10>>>>, numeric: &[u8]) -> Option<BaseUser> {
        for user in users {
            let borrowed = user.borrow();
            if borrowed.ext.numeric == numeric {
                return Some(borrowed.base.clone());
            }
        }

        None
    }

    fn add_local_bot(&self, core_data: &mut NeroData<P10>, bot: &Bot) {
        let mut user_node: User<P10> = User::<P10>::new(&bot.nick.as_bytes(), &bot.ident.as_bytes(), &bot.hostname.as_bytes(), core_data.me.clone());
        user_node.base.ip = "255.255.255.255".into();
        user_node.base.gecos = bot.gecos.as_bytes().to_vec();

        let numeric = get_next_numeric(core_data);
        user_node.ext.numeric = numeric.clone().into_bytes();
        p10_set_user_modes(&mut user_node, "+iok".as_bytes());

        {
            let shared_user = Rc::new(RefCell::new(user_node));
            let mut me_borrow = core_data.me.borrow_mut();
            me_borrow.users.push(shared_user.clone());
            core_data.users.push(shared_user.clone());
        }

        for channel in &bot.channels {
            let timestamp = core_data.now;
            let name = channel.name.clone().into_bytes();
            let modes = channel.chanmodes.clone().into_bytes();
            let mut new_channel = p10_add_channel(core_data, &name, timestamp, &modes, &String::new().into_bytes()).unwrap();
            let member_b = p10_add_channel_member(core_data, &mut new_channel, &numeric.clone().into_bytes()).unwrap();
            let mut member = member_b.borrow_mut();

            for mode in channel.umodes.chars() {
                match mode {
                    'o' => member.base.modes |= MMODE_CHANOP.bits(),
                    'v' => member.base.modes |= MMODE_VOICE.bits(),
                    _ => {},
                }
            }
        }
    }

    fn send_privmsg(&self, users: &Vec<Rc<RefCell<User<P10>>>>, write_buffer: &mut Vec<Vec<u8>>, source: &BaseUser, target: &[u8], message: &[u8]) {
        send_textmessage(users, write_buffer, source, target, message, true);
    }

    fn send_notice(&self, users: &Vec<Rc<RefCell<User<P10>>>>, write_buffer: &mut Vec<Vec<u8>>, source: &BaseUser, target: &[u8], message: &[u8]) {
        send_textmessage(users, write_buffer, source, target, message, false);
    }
}

// Commands

fn p10_cmd_pass(core_data: &mut NeroData<P10>, _origin: &[u8], argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    if argc != 2 {
        return Err(());
    }

    if core_data.uplink.is_some() {
        return Ok(());
    }

    let recv_pass: &[u8] = &argv[1];
    if core_data.config.uplink.recv_pass.as_bytes() != recv_pass {
        log(Error, "MAIN", format!("Uplink password did not match our password"));
    }

    Ok(())
}

fn p10_cmd_server(core_data: &mut NeroData<P10>, origin: &[u8], argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    use std::str;

    if argc < 8 {
        return Err(());
    }

    let mut server: Server<P10> = Server::<P10>::new(&argv[1], &argv[8]);
    server.ext.numeric = vec!(argv[6][0], argv[6][1]);

    match str::from_utf8(&argv[2]) {
        Ok(str_int) => {
            server.base.hops = match String::from(str_int).parse() {
                Ok(i) => i,
                Err(_) => 0,
            };
        },
        Err(_) => {}, // TODO
    }

    match str::from_utf8(&argv[3]) {
        Ok(str_int) => {
            server.base.boot = match String::from(str_int).parse() {
                Ok(i) => i,
                Err(_) => 0,
            };
        },
        Err(_) => {}, // TODO
    }

    match str::from_utf8(&argv[4]) {
        Ok(str_int) => {
            server.base.link_time = match String::from(str_int).parse() {
                Ok(i) => i,
                Err(_) => 0,
            };
        },
        Err(_) => {}, // TODO
    }

    log(Debug, "MAIN", format!("Added server {} with numeric {} and description {}",
        dv(&server.base.hostname), dv(&server.ext.numeric), dv(&server.base.description)));

    let shared_server = Rc::new(RefCell::new(server));

    if core_data.uplink.is_none() {
        core_data.uplink = Some(shared_server.clone());
        p10_burst_our_users(core_data);
    } else {
        let uplink = find_server_numeric(core_data, origin);
        match uplink {
            Some(arc_server) => server.uplink = Some(arc_server.clone()),
            None => {},
        }
    }

    assert!(core_data.uplink.is_some());
    core_data.servers.push(shared_server);
    Ok(())
}

fn p10_cmd_eb(core_data: &mut NeroData<P10>, origin: &[u8]) -> Result<(), ()> {
    let my_uplink = core_data.uplink.clone().unwrap();
    let my_hostname = my_uplink.borrow().base.hostname.clone();
    let sender_rc = match find_server_numeric(core_data, origin).map(|x| x.clone()) {
        Some(server) => server,
        None => return Err(()),
    };

    let mut sender = sender_rc.borrow_mut();

    if sender.base.hostname == my_hostname {
        let eob_message = &p10_irc_eob(core_data);
        let eob_ack_message = &p10_irc_eob_ack(core_data);

        core_data.add_to_buffer(eob_message);
        core_data.add_to_buffer(eob_ack_message);
    }

    sender.ext.self_burst = false;

    Ok(())
}

fn p10_cmd_ea(_core_data: &mut NeroData<P10>, _origin: &[u8]) -> Result<(), ()> {
    Ok(())
}

fn p10_cmd_gl(_core_data: &mut NeroData<P10>, _origin: &[u8], _argc: usize, _argv: &[Vec<u8>]) -> Result<(), ()> {
    Ok(())
}

fn p10_cmd_g(core_data: &mut NeroData<P10>, _origin: &[u8], argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    if argc > 3 {
        let pong_asl_message = &p10_irc_pong_asll(core_data, &argv[2], &argv[3]);
        core_data.add_to_buffer(pong_asl_message);
    }

    Ok(())
}

fn p10_cmd_textmessage(core_data: &mut NeroData<P10>, origin: &[u8], argc: usize, argv: &[Vec<u8>], is_privmsg: bool) -> Result<(), ()> {
    use plugin::HookType::*;
    use plugin::HookData;

    if argc < 2 {
        return Err(());
    }

    let user_option = find_user_numeric(core_data, &origin.to_vec()).map(|x| x.clone());
    if user_option.is_none() {
        return Err(());
    }

    let user = user_option.unwrap();
    let message = &argv[argc-1];
    let target = &argv[1];
    let target_prefix = target[0] as char;

    let hook_type = if target_prefix == '#' || target_prefix == '&' {
        if is_privmsg {
            PrivmsgChan
        } else {
            NoticeChan
        }
    } else {
        if is_privmsg {
            PrivmsgBot
        } else {
            NoticeBot
        }
    };

    let mut hook_data = HookData::new(hook_type.clone());

    let target_key = if hook_type == PrivmsgBot {
        let target_user_option = find_user_numeric(core_data, &target.to_vec()).map(|x| x.clone());
        let target_user = target_user_option.unwrap();
        let borrowed = target_user.borrow();
        borrowed.base.nick.clone()
    } else {
        target.clone()
    };

    hook_data.target = target_key.to_vec();
    hook_data.origin = user.borrow().base.nick.to_vec();
    hook_data.message = message.to_vec();

    core_data.fire_hook(&hook_data);

    Ok(())
}

fn p10_cmd_t(core_data: &mut NeroData<P10>, origin: &[u8], argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    use std::str;

    if argc < 3 {
        return Err(());
    }

    let channel_rc = match find_channel(core_data, &argv[1]).map(|x| x.clone()) {
        Some(c) => c,
        None => return Err(()),
    };

    let topic_time = if argc >= 5 {
        match str::from_utf8(&argv[3]) {
            Ok(str_int) => {
                match String::from(str_int).parse() {
                    Ok(i) => i,
                    Err(_) => 0,
                }
            },
            Err(_) => 0, // TODO
        }
    } else {
        core_data.now
    };

    let option_user = find_user_numeric(core_data, &origin.to_vec()).map(|x| x.clone());
    let mut channel = channel_rc.borrow_mut();
    p10_set_channel_topic(core_data, &mut channel, option_user, &argv[argc-1]);
    channel.base.topic_time = topic_time;

    Ok(())
}

fn p10_cmd_b(core_data: &mut NeroData<P10>, argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    use std::str;

    if argc < 3 {
        return Err(());
    }

    let created_time: u64 = match str::from_utf8(&argv[2]) {
        Ok(str_int) => {
            match String::from(str_int).parse() {
                Ok(i) => i,
                Err(_) => core_data.now,
            }
        },
        Err(_) => core_data.now, // TODO
    };

    let mut next: usize = 3;
    let mut mode_list: Vec<u8> = Vec::new();
    let mut ban_list: Vec<u8> = Vec::new();
    let mut user_list: Vec<u8> = Vec::new();
    let mut n_modes: usize = 1;
    while next < argc {
        match argv[next][0] {
            b'+' => {
                for ii in 1..argv[next].len() {
                    match argv[next][ii] {
                        b'k' | b'l' | b'A' | b'U' => n_modes+=1,
                        _ => {},
                    }
                }

                if next + n_modes > argc {
                    n_modes = argc - next;
                }

                mode_list = unsplit_string(argv, argc, next, n_modes);
                next += n_modes;
            }
            b'%' => {
                ban_list = argv[next][1..argv[next].len()].to_vec();
                next+=1;
            }
            _ => {
                user_list = argv[next].clone();
                next+=1;
            }
        }
    }

    if core_data.unbursted_channels.contains(&argv[1]) {
        let channel = find_channel(core_data, &argv[1]).unwrap();
        p10_burst_our_channel(core_data, created_time, &channel);
    }

    let mut channel = match p10_add_channel(core_data, &argv[1], created_time, &mode_list, &ban_list) {
        Some(channel) => channel,
        None => return Err(()),
    };

    let mut member_modes: u64 = 0;
    let mut oplevel: u64 = 0;
    let mut userbuf: Vec<u8> = Vec::new();
    let mut got_colon: bool = false;
    for (index, &ii) in user_list.iter().enumerate() {
        if ii == b',' || (index > 0 && index + 1 == user_list.len()) {
            if index + 1 == user_list.len() {
                if got_colon {
                    match ii {
                        b'o' => member_modes |= MMODE_CHANOP.bits(),
                        b'v' => member_modes |= MMODE_VOICE.bits(),
                        b'0' ... b'9' => oplevel = 999, // TODO: Parse this
                        _ => {},
                    }
                } else {
                    userbuf.push(ii);
                }
            }

            match p10_add_channel_member(core_data, &mut channel, &userbuf) {
                Ok(member_b) => {
                    let mut member = member_b.borrow_mut();
                    member.base.modes = member_modes;
                    member.ext.oplevel = oplevel;
                    // let user = member.user.borrow();
                    // println!("Set mode={}, oplevel={} for {}", member.base.modes, member.ext.oplevel, dv(&user.base.nick));
                }
                Err(_) => log(Error, "MAIN", format!("Failed to find numeric member {} in channel {}",
                    dv(&userbuf), dv(&channel.borrow().base.name))), // TODO
            }

            userbuf = Vec::new();
            got_colon = false;
            continue;
        }

        if ii == b':' {
            got_colon = true;
            member_modes = 0;
            oplevel = 0;
            continue;
        }

        if got_colon {
            match ii {
                b'o' => member_modes |= MMODE_CHANOP.bits(),
                b'v' => member_modes |= MMODE_VOICE.bits(),
                b'0' ... b'9' => oplevel = 999, // TODO: Parse this
                _ => {},
            }
        } else {
            userbuf.push(ii);
        }
    }

    Ok(())
}

// ABAAB Q :Quit: KVIrc 4.9.2 Aria http://www.kvirc.net/
fn p10_cmd_q(core_data: &mut NeroData<P10>, origin: &[u8], argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    use plugin::HookType::*;
    use plugin::HookData;

    let option_user = find_user_numeric(core_data, &origin.to_vec()).map(|x| x.clone());

    if option_user.is_none() {
        return Err(());
    }

    let user_rc = option_user.unwrap();
    let user = user_rc.borrow();
    let qmessage = &argv[argc-1];

    log(Debug, "MAIN", format!("User {} disconnected from {}: {}",
        dv(&user.base.nick), dv(&user.uplink.borrow().base.hostname), dv(&qmessage)));

    let mut hook_data = HookData::new(UserQuit);
    hook_data.target = user.base.nick.to_vec();
    hook_data.server = Some(user.uplink.borrow().base.clone());
    hook_data.message = qmessage.to_vec();

    core_data.fire_hook(&hook_data);

    p10_del_user(core_data, origin)
}

// AB N SightBlind 1 1496365558 kvirc 127.0.0.1 +owgrh blindsight kvirc@blindsight.users.gamesurge B]AAAB ABAAB :KVIrc 4.9.2 Aria http://kvirc.net/
fn p10_cmd_n(core_data: &mut NeroData<P10>, origin: &[u8], argc: usize, argv: &[Vec<u8>]) -> Result<(), ()> {
    use plugin::HookType::*;
    use plugin::HookData;

    let option_user = find_user_numeric(core_data, &origin.to_vec()).map(|x| x.clone());
    // println!("Looking for nick, argc={}, origin={}", argc, dv(origin));
    if option_user.is_some() {
        // println!("Found user!");
        if argc < 2 {
            return Err(());
        }

        let user = option_user.unwrap();
        log(Debug, "MAIN", format!("User '{}' changing nick to '{}'", dv(&user.borrow().base.nick), dv(&argv[1])));
        user.borrow_mut().base.nick = argv[1].clone();
    } else {
        // println!("Couldnt find user, adding");
        if argc < 9 {
            return Err(());
        }

        let server = find_server_numeric(core_data, &origin.to_vec()).map(|x| x.clone());
        let modes: Vec<u8> = if argc > 9 {
            unsplit_string(argv, argc, 6, argc - 9)
        } else {
            vec!(b'+')
        };

        let user_result = p10_add_user(core_data, server, &argv[1], &argv[4], &argv[5], &modes, &argv[argc-2], &argv[argc-1], &argv[3], &argv[argc-3]);
        match user_result {
            Ok(user_rc) => {
                let user = user_rc.borrow();
                log(Debug, "MAIN", format!("User {} connecting from {}", dv(&user.base.nick), dv(&user.uplink.borrow().base.hostname)));

                let mut hook_data = HookData::new(UserConnected);
                hook_data.target = user.base.nick.to_vec();
                hook_data.server = Some(user.uplink.borrow().base.clone());

                core_data.fire_hook(&hook_data);
            },
            Err(_) => {
                return Err(());
            }
        }
    }

    Ok(())
}

// Helpers

fn p10_set_channel_topic(core_data: &mut NeroData<P10>, channel: &mut RefMut<Channel<P10>>, user: Option<Rc<RefCell<User<P10>>>>, topic: &[u8]) {
    //let old_topic: Vec<u8> = channel.base.topic.to_vec().clone();
    channel.base.topic = topic.to_vec().clone();
    channel.base.topic_time = core_data.now;
    match user {
        Some(u) => {
            channel.base.topic_nick = u.borrow().base.nick.clone();
        },
        None => {},
    }

    // println!("Topic for {} is now {} set by {}", dv(&channel.name), dv(&channel.base.topic), dv(&channel.base.topic_nick));
}

fn p10_add_channel_member(core_data: &mut NeroData<P10>, channel: &mut Rc<RefCell<Channel<P10>>>, userbuf: &[u8]) -> Result<Rc<RefCell<ChannelMember<P10>>>, ()> {
    let user = match find_user_numeric(core_data, &userbuf.to_vec()) {
        Some(u) => u,
        None => return Err(()),
    };

    let mut member = ChannelMember::<P10>::new(user.clone());
    member.base.idle = core_data.now;

    let shared_member = Rc::new(RefCell::new(member));
    let mut c = channel.borrow_mut();
    c.members.push(shared_member.clone());

    if c.members.len() == 1 && c.base.modes & CMODE_REGISTERED.bits() == 0 && c.base.modes & CMODE_APASS.bits() == 0 {
        shared_member.borrow_mut().base.modes |= MMODE_CHANOP.bits();
    }

    log(Debug, "MAIN", format!("Added member {} to channel {}", dv(&user.borrow().base.nick), dv(&c.base.name)));

    Ok(shared_member)
}

fn p10_add_channel(core_data: &mut NeroData<P10>, name: &[u8], created_time: u64, mode_list: &[u8], ban_list: &[u8]) -> Option<Rc<RefCell<Channel<P10>>>> {
    match find_channel(core_data, name) {
        Some(current_channel_rc) => {
            let mut current_channel = current_channel_rc.borrow_mut();
            if current_channel.base.created > created_time {
                current_channel.base.created = created_time;
                current_channel.base.topic_time = 0;
                current_channel.base.topic = Vec::new();
            }

            return Some(current_channel_rc.clone());
        }
        None => {},
    }

    let mut channel = Channel::<P10>::new(name, created_time);

    p10_set_channel_modes(&mut channel, mode_list);
    p10_set_channel_bans(&mut channel, ban_list);

    let shared_channel = Rc::new(RefCell::new(channel));
    core_data.channels.push(shared_channel.clone());

    Some(shared_channel)
}

fn p10_set_channel_bans(channel: &mut Channel<P10>, ban_list: &[u8]) {
    for ban in split_string(ban_list) {
        p10_ban_channel_user(channel, true, &ban);
    }
}

fn p10_set_channel_modes(channel: &mut Channel<P10>, mode_list: &[u8]) {
    use std::str;

    let split_modes: Vec<Vec<u8>> = split_string(mode_list);

    let mut found_modes: u64 = 0;
    let can_set_setmodes = |channel: &Channel<P10>, data: &mut u64, flag: u64| {
        if p10_channel_has_mode(&channel, flag) && *data & flag == 0 {
            *data |= flag;
            return true;
        }

        false
    };

    if split_modes.len() > 0 {
        for jj in 1..split_modes[0].len() {
            p10_add_channel_mode(channel, true, &split_modes[0][jj]);
        }

        for ii in 1..split_modes.len() {
            if can_set_setmodes(&channel, &mut found_modes, CMODE_LIMIT.bits()) {
                channel.base.limit = str::from_utf8(&split_modes[ii]).unwrap().parse().unwrap();
                continue;
            }

            if can_set_setmodes(&channel, &mut found_modes, CMODE_KEY.bits()) {
                channel.base.key = Some(split_modes[ii].clone());
                continue;
            }

            if can_set_setmodes(&channel, &mut found_modes, CMODE_UPASS.bits()) {
                channel.ext.upass = Some(split_modes[ii].clone());
                continue;
            }

            if can_set_setmodes(&channel, &mut found_modes, CMODE_APASS.bits()) {
                channel.ext.apass = Some(split_modes[ii].clone());
                continue;
            }
        }
    }
}

fn p10_ban_channel_user(channel: &mut Channel<P10>, adding: bool, ban: &[u8]) {
    if adding {
        channel.base.bans.push(ban.to_vec().clone());
        // println!("Added ban for {} in {}", dv(&ban), dv(&channel.name));
    } else {
        channel.base.bans.iter().position(|n| n == &ban).map(|e| channel.base.bans.remove(e));
        // println!("Removed ban {} in {}", dv(&ban), dv(&channel.name));
    }
}

fn p10_del_user(core_data: &mut NeroData<P10>, numeric: &[u8]) -> Result<(), ()> {
    use std::str;

    if numeric.len() < 3 || numeric.len() > 5 {
        return Err(())
    }

    let mut idx: usize = 0;
    for user in &core_data.users {
        if &user.borrow().ext.numeric == &numeric.to_vec() {
            break;
        }

        if idx == core_data.users.len() - 1 {
            panic!("Called p10_del_user() but could not find numeric {}", dv(&numeric));
        }

        idx += 1;
    }

    core_data.users.remove(idx);

    let server = find_server_from_user(core_data, &numeric.to_vec()).unwrap();
    idx = 0;
    for user in &server.borrow().users {
        if &user.borrow().ext.numeric == &numeric.to_vec() {
            break;
        }

        if idx == server.borrow().users.len() - 1{
            panic!("Called p10_del_user() but could not find numeric for server {}", dv(&numeric))
        }

        idx += 1;
    }

    server.borrow_mut().users.remove(idx);

    Ok(())
}

fn p10_add_user(core_data: &mut NeroData<P10>, option_uplink: Option<Rc<RefCell<Server<P10>>>>, nick: &[u8], ident: &[u8], hostname: &[u8], modes: &[u8], numeric: &[u8], gecos: &[u8], timestamp: &[u8], realip: &[u8]) -> Result<Rc<RefCell<User<P10>>>, ()> {
    use std::str;

    let decimal_ip = base64_to_vecu8(&realip);
    // println!("Found IP {}, modes, {}", dv(&decimal_ip), dv(&modes));
    // println!("Found user with the following: uplink={:?}, nick={}, ident={}, hostname={}, modes={}, numeric={}, gecos={}, timestamp={}, realip={}",
    //     option_uplink, dv(nick), dv(ident), dv(hostname), dv(modes), dv(numeric), dv(gecos), dv(timestamp), dv(decimal_ip));

    if numeric.len() < 3 || numeric.len() > 5 {
        return Err(())
    }

    if option_uplink.is_none() {
        return Err(())
    }

    let uplink = option_uplink.unwrap();
    // if uplink != find_server_numeric(core_data, numeric) {
    //     return Err(())
    // }

    let mut user_node: User<P10> = User::<P10>::new(&nick, &ident, &hostname, uplink.clone());
    user_node.base.ip = decimal_ip.to_vec();
    user_node.base.gecos = gecos.to_vec();
    user_node.ext.numeric = numeric.to_vec();

    match str::from_utf8(timestamp) {
        Ok(str_int) => {
            user_node.ext.timestamp = match String::from(str_int).parse() {
                Ok(i) => i,
                Err(_) => 0,
            };
        },
        Err(_) => {}, // TODO
    }

    p10_set_user_modes(&mut user_node, modes);

    let shared_user = Rc::new(RefCell::new(user_node));
    uplink.borrow_mut().users.push(shared_user.clone());
    core_data.users.push(shared_user.clone());

    Ok(shared_user.clone())
}

fn p10_add_channel_mode(channel: &mut Channel<P10>, adding: bool, mode: &u8) {
    match mode {
        &b'p' => p10_set_channel_mode_helper(channel, adding, CMODE_PRIVATE.bits()),
        &b's' => p10_set_channel_mode_helper(channel, adding, CMODE_SECRET.bits()),
        &b'm' => p10_set_channel_mode_helper(channel, adding, CMODE_MODERATED.bits()),
        &b't' => p10_set_channel_mode_helper(channel, adding, CMODE_TOPICLIMIT.bits()),
        &b'i' => p10_set_channel_mode_helper(channel, adding, CMODE_INVITEONLY.bits()),
        &b'n' => p10_set_channel_mode_helper(channel, adding, CMODE_NOPRIVMSGS.bits()),
        &b'k' => p10_set_channel_mode_helper(channel, adding, CMODE_KEY.bits()),
        &b'b' => p10_set_channel_mode_helper(channel, adding, CMODE_BAN.bits()),
        &b'l' => p10_set_channel_mode_helper(channel, adding, CMODE_LIMIT.bits()),
        &b'D' => p10_set_channel_mode_helper(channel, adding, CMODE_DELAYJOINS.bits()),
        &b'r' => p10_set_channel_mode_helper(channel, adding, CMODE_REGONLY.bits()),
        &b'c' => p10_set_channel_mode_helper(channel, adding, CMODE_NOCOLORS.bits()),
        &b'C' => p10_set_channel_mode_helper(channel, adding, CMODE_NOCTCPS.bits()),
        &b'z' => p10_set_channel_mode_helper(channel, adding, CMODE_REGISTERED.bits()),
        &b'A' => p10_set_channel_mode_helper(channel, adding, CMODE_APASS.bits()),
        &b'U' => p10_set_channel_mode_helper(channel, adding, CMODE_UPASS.bits()),
        _ => {},
    }
}

fn p10_set_channel_mode_helper(channel: &mut Channel<P10>, adding: bool, flag: u64) {
    if adding {
        channel.base.modes |= flag;
        // println!("Channel {} adding mode {}", dv(&channel.name), *mode as char);
    } else {
        channel.base.modes &= flag;
        // println!("Channel {} removing mode {}", dv(&channel.name), *mode as char);
    }
}

fn p10_channel_has_mode(channel: &Channel<P10>, flag: u64) -> bool {
    channel.base.modes & flag > 0
}

fn p10_set_user_modes(user: &mut User<P10>, modes: &[u8]) {
    let mut adding: bool = true;
    let mut wordptr: usize = 0;

    while wordptr < modes.len() && modes[wordptr] != b' ' {
        wordptr+=1;
    }

    while wordptr < modes.len() && modes[wordptr] == b' ' {
        wordptr+=1;
    }

    for mode in modes {
        match mode {
            &b' ' => break,
            &b'+' => adding = true,
            &b'-' => adding = false,
            &b'o' => p10_set_user_mode_helper(user, adding, UMODE_OPER.bits()),
            &b'i' => p10_set_user_mode_helper(user, adding, UMODE_INVISIBLE.bits()),
            &b'w' => p10_set_user_mode_helper(user, adding, UMODE_WALLOP.bits()),
            &b'd' => p10_set_user_mode_helper(user, adding, UMODE_DEAF.bits()),
            &b'k' => p10_set_user_mode_helper(user, adding, UMODE_SERVICE.bits()),
            &b'g' => p10_set_user_mode_helper(user, adding, UMODE_GLOBAL.bits()),
            &b'n' => p10_set_user_mode_helper(user, adding, UMODE_NOCHAN.bits()),
            &b'I' => p10_set_user_mode_helper(user, adding, UMODE_NOIDLE.bits()),
            &b'x' => p10_set_user_mode_helper(user, adding, UMODE_HIDDEN_HOST.bits()),
            &b'r' => {
                if wordptr > 0 {
                    let mut tag: Vec<u8> = Vec::new();

                    while wordptr < modes.len() && modes[wordptr] != b' ' && modes[wordptr] != b':' {
                        tag.push(modes[wordptr]);
                        wordptr+=1;
                    }

                    if modes[wordptr] == b':' {
                        // let mut another_colon: bool = false;
                        let mut tmpbuf: Vec<u8> = Vec::new();
                        let mut accum: usize = 0;
                        for index in wordptr..modes.len() {
                            match modes[index] {
                                b'0' ... b'9' => {
                                    tmpbuf.push(modes[index]);
                                    accum+=1;
                                }
                                b':' => {
                                    // another_colon = true;
                                    accum+=1;
                                    break;
                                }
                                _ => {
                                    accum+=1;
                                    break;
                                }
                            }
                        }

                        wordptr+=accum;
                    }

                    while wordptr < modes.len() && modes[wordptr] == b' ' {
                        wordptr+=1;
                    }

                    p10_set_user_mode_helper(user, adding, UMODE_STAMPED.bits());
                    user.base.account = tag;
                }
            }
            &b'h' => {
                if wordptr > 0 {
                    let mut mask: Vec<u8> = Vec::new();
                    while wordptr < modes.len() && modes[wordptr] != b' ' {
                        mask.push(modes[wordptr]);
                        wordptr+=1;
                    }

                    while wordptr < modes.len() && modes[wordptr] == b' ' {
                        wordptr+=1;
                    }

                    let mut got_at: bool = false;
                    let mut back: Vec<u8> = Vec::new();
                    let mut front: Vec<u8> = Vec::new();
                    for character in mask {
                        if character == b'@' && ! got_at {
                            got_at = true;
                            continue;
                        }

                        if got_at {
                            back.push(character);
                        } else {
                            front.push(character);
                        }
                    }

                    if back.len() > 0 {
                        user.ext.fakeident = front;
                        user.ext.fakehost = back;
                        // println!("Set fakehost for '{}' to '{}'@'{}'", dv(&user.base.nick), dv(&user.ext.fakeident), dv(&user.ext.fakehost));
                    } else {
                        user.ext.fakehost = front;
                        // println!("Set fakehost for '{}' to '{}'", dv(&user.base.nick), dv(&user.ext.fakehost));
                    }
                }
            }
            _ => {
                log(Error, "MAIN", format!("Got unknown mode {} for user {}", dv(&user.base.nick), *mode as char));
            }
        }
    }
}

fn p10_set_user_mode_helper(user: &mut User<P10>, adding: bool, flag: u64) {
    if adding {
        user.base.modes |= flag;
        // println!("User {} adding mode {}", dv(&user.base.nick), *mode as char);
    } else {
        user.base.modes &= flag;
        // println!("User {} removing mode {}", dv(&user.base.nick), *mode as char);
    }
}

fn send_textmessage(users: &Vec<Rc<RefCell<User<P10>>>>, write_buffer: &mut Vec<Vec<u8>>, source: &BaseUser, target: &[u8], message: &[u8], is_privmsg: bool) {
    if let Some(u) = find_user_nick(users, &source.nick) {
        let borrowed = u.borrow();
        let numeric = borrowed.ext.numeric.clone();

        if numeric.is_empty() {
            panic!("No numeric specified in source user {}", dv(&source.nick));
        }

        let sendfunc = if is_privmsg { p10_irc_privmsg } else { p10_irc_notice };
        let mut send_target = target.to_vec();

        // FIXME
        // This does not take in to account that a user could have their nickname set as a
        // numnick for another user.
        if let Some(t) = find_user_nick(users, &target.to_vec()) {
            let borrowed_target = t.borrow();
            send_target = borrowed_target.ext.numeric.clone();
        }

        sendfunc(write_buffer, &numeric, &send_target, message);
    } else {
        log(Error, "P10", format!("Sending message for a user that doesn't exist! {}", dv(&source.nick)));
    }
}


fn find_channel(core_data: &NeroData<P10>, name: &[u8]) -> Option<Rc<RefCell<Channel<P10>>>> {
    let lower: &[u8] = &u8_slice_to_lower(name);

    for channel in &core_data.channels {
        if &channel.borrow().base.name as &[u8] == lower {
            return Some(channel.clone());
        }
    }

    None
}

fn find_server_numeric<'a>(core_data: &'a NeroData<P10>, numeric: &[u8]) -> Option<&'a Rc<RefCell<Server<P10>>>> {
    for server in &core_data.servers {
        if &server.borrow().ext.numeric as &[u8] == numeric {
            return Some(server);
        }
    }

    None
}

fn find_server_from_user(core_data: &NeroData<P10>, numeric: &Vec<u8>) -> Option<Rc<RefCell<Server<P10>>>> {
    let mut lookup_numeric = numeric.clone();
    while lookup_numeric.len() > 2 {
        lookup_numeric.pop();
    }

    assert!(lookup_numeric.len() == 2);

    for server in &core_data.servers {
        if server.borrow().ext.numeric == lookup_numeric {
            return Some(server.clone());
        }
    }

    None
}

fn find_user_numeric<'a>(core_data: &'a NeroData<P10>, numeric: &Vec<u8>) -> Option<&'a Rc<RefCell<User<P10>>>> {
    for user in &core_data.users {
        if &user.borrow().ext.numeric == numeric {
            return Some(user);
        }
    }

    None
}

fn find_user_nick(users: &Vec<Rc<RefCell<User<P10>>>>, nick: &Vec<u8>) -> Option<Rc<RefCell<User<P10>>>> {
    for user in users {
        if &user.borrow().base.nick == nick {
            return Some(user.clone())
        }
    }

    None
}

fn get_next_numeric(core_data: &mut NeroData<P10>) -> String {
    let local_numeric = String::from_utf8(core_data.me.borrow().ext.numeric.clone()).unwrap();
    let mut uplink = core_data.me.borrow_mut();

    assert!(local_numeric.len() > 0);

    let numnick = inttobase64(uplink.ext.numeric_accum as usize, 3);

    uplink.ext.numeric_accum += 1;
    format!("{}{}", local_numeric, numnick)
}

fn p10_build_channel_mode_string(modes: u64, limit: u64, key_option: &Option<Vec<u8>>, ext: &P10ChannelExt) -> String {
    static P10_CHANNEL_MODES: &'static [u8] = b"psmtinkblDrcCzAu";
    let mut buf: Vec<u8> = Vec::new();

    for ii in 0..15 {
        if modes & (1 << ii) > 0 {
            buf.push(P10_CHANNEL_MODES[ii]);
        }
    }

    let mut buf = String::from_utf8(buf).unwrap();

    if limit > 0 {
        assert!(modes & CMODE_LIMIT.bits() > 0);
        buf = format!("{} {}", buf, limit);
    }

    if let Some(ref key) = *key_option {
        assert!(modes & CMODE_KEY.bits() > 0);
        buf = format!("{} {}", buf, dv(&key));
    }

    if let Some(ref upass) = ext.upass {
        assert!(modes & CMODE_UPASS.bits() > 0);
        buf = format!("{} {}", buf, dv(&upass));
    }

    if let Some(ref apass) = ext.apass {
        assert!(modes & CMODE_APASS.bits() > 0);
        buf = format!("{} {}", buf, dv(&apass));
    }

    buf
}

fn p10_burst_our_channel(core_data: &mut NeroData<P10>, created: u64, channel_rc: &Rc<RefCell<Channel<P10>>>) {
    let channel = channel_rc.borrow();
    let local_numeric = String::from_utf8(core_data.me.borrow().ext.numeric.clone()).unwrap();

    let base_burst = format!("{} B {} {} ", local_numeric, dv(&channel.base.name), created);
    let chan_modes = p10_build_channel_mode_string(channel.base.modes, channel.base.limit, &channel.base.key, &channel.ext);
    let mut burst_message = base_burst.clone() + "+" + &chan_modes + " ";

    let mut was_opped = false;
    let mut was_voiced = false;

    for member_rc in &channel.members {
        let member = &member_rc.borrow();
        let user = &member.user.borrow();

        log(Debug, "MAIN", format!("Adding local member {} to channel {}", dv(&user.base.nick), dv(&channel.base.name)));
        let mut need_colon = false;
        let mut oplen = 0;

        if member.base.modes & MMODE_CHANOP.bits() > 0 && ! was_opped {
            need_colon = true;
            was_opped = true;
            oplen +=1;
        }

        if member.base.modes & MMODE_VOICE.bits() > 0 && ! was_voiced {
            need_colon = true;
            was_voiced = true;
            oplen +=1;
        }

        if member.base.modes & MMODE_CHANOP.bits() == 0 && was_opped {
            need_colon = true;
            was_opped = false;
        }

        if member.base.modes & MMODE_VOICE.bits() == 0 && was_voiced {
            need_colon = true;
            was_voiced = false;
        }

        if need_colon {
            oplen +=1;
        }

        if burst_message.len() + user.ext.numeric.len() + oplen + 1 >= 500 {
            core_data.write_buffer.push(burst_message.into_bytes());
            burst_message = base_burst.clone();
        }

        burst_message = format!("{}{}", burst_message, dv(&user.ext.numeric));
        if need_colon {
            burst_message += ":";
            if member.base.modes & MMODE_CHANOP.bits() > 0 {
                burst_message += "o";
            }

            if member.base.modes & MMODE_VOICE.bits() > 0 {
                burst_message += "v";
            }
        }

        burst_message += ",";
    }

    burst_message.pop();

    let mut first_ban = false;
    for ban in &channel.base.bans {
        if burst_message.len() + ban.len() + 2 >= 500 {
            core_data.write_buffer.push(burst_message.into_bytes());
            burst_message = base_burst.clone();
            first_ban = true;
        }
        if first_ban {
            burst_message += ":%";
        }

        burst_message = format!("{} ", dv(&ban));
        first_ban = false;
    }

    if burst_message.len() != base_burst.len() {
        core_data.write_buffer.push(burst_message.into_bytes());
    }
}

fn p10_burst_our_users(core_data: &mut NeroData<P10>) {
    let numeric = p10_get_numeric(core_data);
    let now = core_data.now;

    for user in &core_data.me.borrow().users {
        p10_irc_user(&numeric, now, &*user.borrow(), &mut core_data.write_buffer);
    }

    for channel in &core_data.channels {
        let lowered_name = u8_slice_to_lower(&channel.borrow().base.name.clone());

        if core_data.unbursted_channels.contains(&lowered_name) {
            continue;
        }

        core_data.unbursted_channels.push(lowered_name);
    }
}

// IRC Command builders
fn p10_get_numeric(core_data: &NeroData<P10>) -> String {
    let numeric_optional = core_data.config.uplink.numeric.clone();
    numeric_optional.unwrap()
}

fn p10_irc_user(numeric: &str, now: u64, user: &User<P10>, buffer: &mut Vec<Vec<u8>>) {
    buffer.push(format!("{} N {} 1 {} {} {} +iok _ {} :{}",
        numeric, dv(&user.base.nick), now, dv(&user.base.ident),
        dv(&user.base.host), dv(&user.ext.numeric), dv(&user.base.gecos)).into_bytes());
}

fn p10_irc_eob(core_data: &NeroData<P10>) -> Vec<u8> {
    let numeric = p10_get_numeric(core_data);

    format!("{} EB", numeric).into_bytes()
}

fn p10_irc_eob_ack(core_data: &NeroData<P10>) -> Vec<u8> {
    let numeric = p10_get_numeric(core_data);

    format!("{} EA", numeric).into_bytes()
}

fn p10_irc_pong_asll(core_data: &NeroData<P10>, who: &[u8], orig_ts: &[u8]) -> Vec<u8> {
    let numeric = p10_get_numeric(core_data);

    format!("{} Z {} {} 0 {}", numeric, dv(&who), dv(&orig_ts), dv(&orig_ts)).into_bytes()
}

fn p10_irc_textmessage(buffer: &mut Vec<Vec<u8>>, source: &[u8], target: &[u8], message: &[u8], cmd: char) {
    let prefix = format!("{} {} {} :", dv(&source), cmd, dv(&target));
    let message_count = ceiling_division(message.len() + prefix.len(), 500);

    for ii in 0..message_count {
        let begin = ii * 500;
        let end = if (ii + 1) * 500 > message.len() {
            message.len() + (ii * 500)
        } else {
            (ii + 1) * 500
        };

        buffer.push(format!("{}{}", prefix, dv(&message[begin..end])).into());
    }
}

fn p10_irc_privmsg(buffer: &mut Vec<Vec<u8>>, source: &[u8], target: &[u8], message: &[u8]) {
    p10_irc_textmessage(buffer, source, target, message, 'P');
}

fn p10_irc_notice(buffer: &mut Vec<Vec<u8>>, source: &[u8], target: &[u8], message: &[u8]) {
    p10_irc_textmessage(buffer, source, target, message, 'O');
}

// murder this
fn split_line(line: &[u8], irc_colon: bool, argv_size: usize) -> (usize, Vec<Vec<u8>>) {
    let mut argc: usize = 0;
    let mut argv: Vec<Vec<u8>> = Vec::new();
    let mut ii = 0;

    while ii < line.len() && argc < argv_size {
        while ii < line.len() && line[ii] == b' ' {
            ii+=1;
        }

        if ii >= line.len() {
            break;
        }

        let jj = ii;
        if line[ii] == b':' && irc_colon && argc > 0 {
            if line.len() > jj+1 {
                argv.push(line[jj+1..].to_owned());
                argc+=1;
            }

            break;
        }

        while ii < line.len() && line[ii] != b' ' {
            ii+=1;
        }

        let mut buf: Vec<u8> = Vec::new();
        for ll in jj..line.len() {
            if ll < ii || line[ll] != b' ' {
                buf.push(line[ll]);
            } else {
                break;
            }
        }

        if buf.len() > 0 {
            argv.push(buf);
            argc+=1;
        }

        if argc >= argv_size {
            break;
        }
    }

    (argc, argv)
}

fn base64_to_vecu8(input: &[u8]) -> Vec<u8> {
    use base64::decode;

    if input == b"_" {
        return Vec::new();
    }

    let mut buffer: Vec<u8> = input.to_vec().clone();
    buffer.push(b'A');
    buffer.push(b'A');

    for ii in 0..buffer.len() {
        if buffer[ii] == b'[' {
            buffer[ii] = b'+';
        } else if buffer[ii] == b']' {
            buffer[ii] = b'/';
        }
    }

    let decoded = match decode(&buffer) {
        Ok(o) => o,
        Err(e) => {
            log(Error, "MAIN", format!("Error decoding {}: {}", dv(&input), e));
            Vec::new()
        }
    };

    if decoded.len() == 0 {
        return Vec::new();
    }

    let ip = vec!(
        (decoded[0] & 0xf) << 4 | (decoded[1] >> 4) & 0xf,
        (decoded[1] & 0xf) << 4 | (decoded[2] >> 4) & 0xf,
        (decoded[2] & 0xf) << 4 | (decoded[3] >> 4) & 0xf,
        (decoded[3] & 0xf) << 4 | (decoded[4] >> 4) & 0xf,
    );

    let stringbuf = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);

    stringbuf.into_bytes()
}

// Tests

#[cfg(test)]
fn test_make_channel() -> Channel<P10> {
    Channel::<P10>::new(format!("#nero").as_bytes(), 0)
}

#[cfg(test)]
fn test_make_shared_server() -> Rc<RefCell<Server<P10>>> {
    let server_hostname: &[u8] = &String::from("test.server").into_bytes();
    let server_description: &[u8] = &String::from("This is a test").into_bytes();
    let server: Server<P10> = Server::<P10>::new(server_hostname, server_description);
    Rc::new(RefCell::new(server))
}

#[cfg(test)]
fn test_make_user() -> User<P10> {
    let nick: &[u8] = &String::from("test").into_bytes();
    let ident: &[u8] = &String::from("kvirc").into_bytes();
    let hostname: &[u8] = &String::from("some.host.name").into_bytes();
    let uplink = test_make_shared_server();

    User::<P10>::new(nick, ident, hostname, uplink)
}

#[test]
fn test_set_user_modes() {
    let mut user = test_make_user();

    let mode_string: &[u8] = &String::from("+owgrh blindsight someu@someh").into_bytes();
    p10_set_user_modes(&mut user, mode_string);

    assert!(user.base.modes & UMODE_STAMPED.bits() > 0);
    assert!(user.base.modes & UMODE_OPER.bits() > 0);
    assert!(user.base.modes & UMODE_GLOBAL.bits() > 0);
    assert!(user.base.modes & UMODE_HIDDEN_HOST.bits() == 0);

    let mode_string: &[u8] = &String::from("+x").into_bytes();
    p10_set_user_modes(&mut user, mode_string);
    assert!(user.base.modes & UMODE_HIDDEN_HOST.bits() > 0);
    assert!(user.base.modes & UMODE_STAMPED.bits() > 0);
    assert!(user.base.modes & UMODE_OPER.bits() > 0);
    assert!(user.base.modes & UMODE_GLOBAL.bits() > 0);
}

#[test]
fn test_parses_channel_bans() {
    let mut channel = test_make_channel();
    let bans_string: &[u8] = &String::from("*!*@test.host.a *ident~!*@* *!*@127.0.0.1").into_bytes();
    p10_set_channel_bans(&mut channel, bans_string);
    assert_eq!(channel.base.bans.len(), 3);
    assert!(channel.base.bans.iter().position(|n| n == &format!("*!*@test.host.a").into_bytes().to_vec()).is_some());
    assert!(channel.base.bans.iter().position(|n| n == &format!("*ident~!*@*").into_bytes().to_vec()).is_some());
    assert!(channel.base.bans.iter().position(|n| n == &format!("*!*@127.0.0.1").into_bytes().to_vec()).is_some());
    assert!(channel.base.bans.iter().position(|n| n == &format!("*!*@*").into_bytes().to_vec()).is_none());

    let mut channel = test_make_channel();
    let bans_string: &[u8] = &String::from("").into_bytes();
    p10_set_channel_bans(&mut channel, bans_string);
    assert_eq!(channel.base.bans.len(), 0);
    assert!(channel.base.bans.iter().position(|n| n == &format!("*!*@*").into_bytes().to_vec()).is_none());
}

#[test]
fn test_parses_channel_mode_strings() {
    let mut channel = test_make_channel();
    let mode_string: &[u8] = &String::from("+ntl 34").into_bytes();
    p10_set_channel_modes(&mut channel, mode_string);
    assert_eq!(channel.base.modes, CMODE_LIMIT.bits() | CMODE_NOPRIVMSGS.bits() | CMODE_TOPICLIMIT.bits());
    assert_eq!(channel.base.limit, 34);

    let mut channel = test_make_channel();
    assert_eq!(channel.base.modes, 0);
    let mode_string: &[u8] = &String::from("+kU THAKEY userpass").into_bytes();
    p10_set_channel_modes(&mut channel, mode_string);
    assert!(channel.base.key.is_some());
    assert!(channel.ext.upass.is_some());
    let key = &channel.base.key.unwrap();
    let upass = &channel.ext.upass.unwrap();
    assert_eq!(key, b"THAKEY");
    assert_eq!(upass, b"userpass");
    assert_eq!(channel.base.modes, CMODE_KEY.bits() | CMODE_UPASS.bits());
}

#[test]
fn test_adds_channel_mode_bitflags() {
    let mut channel = test_make_channel();

    // Private
    assert!(channel.base.modes & CMODE_PRIVATE.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'p');
    assert!(channel.base.modes & CMODE_PRIVATE.bits() > 0);

    // Secret
    assert!(channel.base.modes & CMODE_SECRET.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b's');
    assert!(channel.base.modes & CMODE_MODERATED.bits() | CMODE_SECRET.bits() > 0);

    // Moderated
    assert!(channel.base.modes & CMODE_MODERATED.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'm');
    assert!(channel.base.modes & CMODE_MODERATED.bits() > 0);

    // Topic limit
    assert!(channel.base.modes & CMODE_TOPICLIMIT.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b't');
    assert!(channel.base.modes & CMODE_TOPICLIMIT.bits() > 0);

    // Invite only
    assert!(channel.base.modes & CMODE_INVITEONLY.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'i');
    assert!(channel.base.modes & CMODE_INVITEONLY.bits() > 0);

    // No outside private messages
    assert!(channel.base.modes & CMODE_NOPRIVMSGS.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'n');
    assert!(channel.base.modes & CMODE_NOPRIVMSGS.bits() > 0);
    assert!(channel.base.modes & CMODE_NOPRIVMSGS.bits() & CMODE_LIMIT.bits() == 0);

    // Channel has a key
    assert!(channel.base.modes & CMODE_KEY.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'k');
    assert!(channel.base.modes & CMODE_KEY.bits() > 0);

    // Ban
    assert!(channel.base.modes & CMODE_BAN.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'b');
    assert!(channel.base.modes & CMODE_BAN.bits() > 0);

    // Channel has a limit
    assert!(channel.base.modes & CMODE_LIMIT.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'l');
    assert!(channel.base.modes & CMODE_LIMIT.bits() > 0);

    // Channel is delayed join
    assert!(channel.base.modes & CMODE_DELAYJOINS.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'D');
    assert!(channel.base.modes & CMODE_DELAYJOINS.bits() > 0);

    // Channel is only allowing registered users
    assert!(channel.base.modes & CMODE_REGONLY.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'r');
    assert!(channel.base.modes & CMODE_REGONLY.bits() > 0);

    // Channel is blocking color codes
    assert!(channel.base.modes & CMODE_NOCOLORS.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'c');
    assert!(channel.base.modes & CMODE_NOCOLORS.bits() > 0);

    // Channel is blocking CTCPs
    assert!(channel.base.modes & CMODE_NOCTCPS.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'C');
    assert!(channel.base.modes & CMODE_NOCTCPS.bits() > 0);

    // Channel is registered
    assert!(channel.base.modes & CMODE_REGISTERED.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'z');
    assert!(channel.base.modes & CMODE_REGISTERED.bits() > 0);

    // Channel has an admin password
    assert!(channel.base.modes & CMODE_APASS.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'A');
    assert!(channel.base.modes & CMODE_APASS.bits() > 0);

    // Channel has a user password
    assert!(channel.base.modes & CMODE_UPASS.bits() == 0);
    p10_add_channel_mode(&mut channel, true, &b'U');
    assert!(channel.base.modes & CMODE_UPASS.bits() > 0);
}
