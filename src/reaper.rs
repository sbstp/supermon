use std::thread;

use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd;

use crate::events::{Event, EventSender};

pub fn start(sender: EventSender) {
    thread::spawn(move || loop {
        if let Ok(status) = waitpid(unistd::Pid::from_raw(-1), None) {
            match status {
                WaitStatus::Exited(pid, code) => {
                    if sender.send(Event::Exited(pid.into(), code)).is_err() {
                        break;
                    }
                }
                WaitStatus::Signaled(pid, signal, _) => {
                    if sender.send(Event::Signaled(pid.into(), signal)).is_err() {
                        break;
                    }
                }
                _ => {}
            }
        }
    });
}
