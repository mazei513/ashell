use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use super::{Module, OnModulePress};
use crate::app;
use iced::{time::every, widget::text, Element, Subscription};
use itertools::Itertools;
use log::{debug, error};

fn connect_mpd<A: ToSocketAddrs>(addr: A) -> TcpStream {
    debug!("connecting to mpd");
    let client = TcpStream::connect(addr).unwrap();
    let v = read_mpd_response(&client).unwrap();
    for l in v {
        debug!("mpd connected: {}", l)
    }
    client
}

fn read_mpd_response(client: &TcpStream) -> Result<Vec<String>, String> {
    let br = BufReader::new(client);
    let mut ls = Vec::<String>::new();
    for l in br.lines() {
        match l {
            Ok(l) => {
                debug!("mpd response: {}", l);
                if l.starts_with("OK") {
                    ls.push(l);
                    break;
                }
                if l.starts_with("ACK") {
                    error!("mpd ACK response: {}", l);
                    return Err(l);
                }
                ls.push(l);
            }
            Err(err) => {
                return Err(format!("mpd response error: {}", err));
            }
        }
    }
    Ok(ls)
}

fn read_current_song_response(client: &TcpStream) -> Option<String> {
    match read_mpd_response(client) {
        Ok(r) => Some(
            r.iter()
                .filter_map(|f| f.strip_prefix("Artist:").or(f.strip_prefix("Title:")))
                .join(" - "),
        ),
        Err(err) => {
            error!("{}", err);
            None
        }
    }
}

fn get_current_song(client: &mut TcpStream) -> Option<String> {
    debug!("fetching current song");
    match client.write(b"currentsong\n") {
        Ok(_) => {
            debug!("mpd responded");
            read_current_song_response(client)
        }
        Err(_) => None,
    }
}

pub struct Mpd {
    client: TcpStream,
    value: Option<String>,
}

impl Mpd {
    pub fn new(addr: String) -> Self {
        let mut client = connect_mpd(addr);
        let value = get_current_song(&mut client);
        debug!("mpd module initialized");
        Mpd { client, value }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl Mpd {
    pub fn update(&mut self, message: Message, max_length: usize) {
        match message {
            Message::Update => {
                self.value = get_current_song(&mut self.client).map(|mut s| {
                    s.truncate(max_length);
                    s
                });
            }
        }
    }
}

impl Module for Mpd {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        (): Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        self.value
            .as_ref()
            .map(|value| (text(value).size(12).into(), None))
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(
            every(Duration::from_secs(1))
                .map(|_| Message::Update)
                .map(app::Message::Mpd),
        )
    }
}
