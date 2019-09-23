use std::io;
use std::process::ExitStatus;
use std::sync::Arc;

use crossbeam_channel::{Sender, Receiver};

use crate::spec::AppInfo;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub enum EventKind {
    Exit(ExitStatus),
    SpawnError(io::Error),
    WaitError(io::Error),
    Line(StreamKind, Vec<u8>),
    Err(StreamKind, io::Error),
    Eof(StreamKind),
}

#[derive(Debug)]
pub struct Event {
    pub app: Arc<AppInfo>,
    pub kind: EventKind,
}

impl Event {
    pub fn new(app: &Arc<AppInfo>, kind: EventKind) -> Event {
        Event {
            app: app.clone(),
            kind: kind,
        }
    }
}

pub type EventSender = Sender<Event>;
pub type EventReceiver = Receiver<Event>;
