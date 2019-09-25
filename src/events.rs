use std::io;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use nix::sys::signal::Signal;

use crate::spec::AppInfo;
use crate::utils::Pid;


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub enum EventKind {
    Started(Pid),
    SpawnError(io::Error),
    Line(StreamKind, Vec<u8>),
    Err(StreamKind, io::Error),
    Eof(StreamKind),
}

#[derive(Debug)]
pub enum Event {
    App { app: Arc<AppInfo>, kind: EventKind },
    Signal(Signal),
    Exited(Pid, i32),
    Signaled(Pid, Signal),
}

impl Event {
    pub fn new(app: &Arc<AppInfo>, kind: EventKind) -> Event {
        Event::App {
            app: app.clone(),
            kind: kind,
        }
    }
}

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;
