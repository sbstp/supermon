use std::io;
use std::sync::Arc;
use std::fmt;

use crossbeam_channel::{Receiver, Sender};
use nix::sys::signal::Signal;
use nix::unistd::{self};

use crate::spec::AppInfo;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Pid(pub u32);

impl From<unistd::Pid> for Pid {
    fn from(pid: unistd::Pid) -> Pid {
        let raw = pid.as_raw();
        if raw < 0 {
            panic!("events: negative PID");
        }
        Pid(raw as u32)
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
