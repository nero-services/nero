use std::collections::VecDeque;
use std::io::{self, BufReader};

use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_io::AsyncRead;
use tokio_io::io::{ReadHalf, WriteHalf, read_until, write_all};

use futures::{BoxFuture, Future};
use futures::future::{Loop, loop_fn};

use config;
use core_data::NeroData;
use protocol::Protocol;

#[derive(Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Quitting,
    Connecting,
    Bursting,
    Connected,
}

#[derive(Debug)]
pub struct WriteState {
    messages: Vec<Vec<u8>>,
    writer:WriteHalf<TcpStream>,
}

pub struct NetState<P: Protocol> {
    core_data: NeroData<P>,
    protocol: P,
}

impl<P: Protocol> NetState<P> {
    pub fn new(config: config::Config) -> Self {
        Self {
            core_data: NeroData::<P>::new(config),
            protocol: P::new(),
        }
    }

    pub fn start_handshake(&mut self, messages: &mut Vec<Vec<u8>>) {
        self.protocol.start_handshake(&mut self.core_data, messages);
    }

    pub fn process(&mut self, buffer: &mut Vec<u8>, messages: &mut Vec<Vec<u8>>) {
        {
            let message: &[u8] = trim_bytes_right(&buffer);
            println!("   {}", String::from_utf8_lossy(message).chars().filter(|c| ! c.is_control()).collect::<String>());
            self.protocol.process(message, &mut self.core_data, messages);
        }

        buffer.clear();
    }
}

impl WriteState {
    pub fn new(writer: WriteHalf<TcpStream>) -> Self {
        Self {
            messages: Vec::new(),
            writer: writer,
        }
    }

    pub fn messages_mut(&mut self) -> &mut Vec<Vec<u8>> {
        &mut self.messages
    }

    pub fn write_lines(self) -> BoxFuture<Self, io::Error> {
        use futures::future::ok;

        loop_fn((self.messages.into(), self.writer), |(mut messages, writer): (VecDeque<Vec<u8>>, _)| {
            match messages.pop_front() {
                Some(mut message) => {
                    println!("W: {}", String::from_utf8_lossy(&message));
                    if message.iter().next_back() != Some(&b'\n') {
                        message.push(b'\n');
                    }

                    write_all(writer, message).map(|(writer, _)| {
                        Loop::Continue((messages, writer))
                    }).boxed()
                },
                None => {
                    messages.clear();
                    ok(Loop::Break(WriteState { messages: messages.into(), writer })).boxed()
                }
            }
        }).boxed()
    }
}

pub fn trim_bytes_right(mut input: &[u8]) -> &[u8] {
    loop {
        match input.iter().next_back() {
            Some(&b'\r') | Some(&b'\n') => {
                input = &input[0..input.len()-1]
            }
            _ => break,
        }
    }

    input
}

pub fn boot<P: Protocol>(handle: Handle) -> Box<Future<Item=(), Error=io::Error>> {
    let cfg_opt1 = config::load();
    let config_data = match cfg_opt1 {
        Ok(cfg_parsed) => {
            match cfg_parsed {
                Ok(cfg) => cfg,
                Err(e) => panic!("Failed to read config file: {}", e),
            }
        },
        Err(e) => panic!("Failed to load config file: {}", e),
    };

    let mut net_state = NetState::<P>::new(config_data);
    let addr = format!("{}:{}", net_state.core_data.config.uplink.ip, net_state.core_data.config.uplink.port).parse().unwrap();

    net_state.core_data.load_plugins();

    Box::new(TcpStream::connect(&addr, &handle).and_then(|stream| {
        let (reader, writer) = stream.split();
        let reader: BufReader<ReadHalf<_>> = BufReader::new(reader);

        let mut write_state = WriteState::new(writer);

        net_state.start_handshake(write_state.messages_mut());
        write_state.write_lines().and_then(move |write_state| {
            loop_fn((Vec::new(), reader, write_state, net_state), move |(buffer, reader, mut write_state, mut net_state)| {
                read_until(reader, b'\n', buffer).and_then(move |(reader, mut buffer)| {

                    net_state.process(&mut buffer, write_state.messages_mut());
                    write_state.write_lines().map(|write_state| {
                        Loop::Continue((buffer, reader, write_state, net_state))
                    })
                })
            })
        })
    }))
}
