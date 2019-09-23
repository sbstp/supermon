use std::thread;

use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd;

use crate::events::{Event, EventSender, Pid};

pub fn start(sender: EventSender) {
    thread::spawn(move || loop {
        if let Ok(status) = waitpid(unistd::Pid::from_raw(-1), None) {
            match status {
                WaitStatus::Exited(pid, code) => {
                    sender.send(Event::Exited(pid.into(), code));
                }
                WaitStatus::Signaled(pid, signal, core_dumped) => {}
                _ => {}
            }
        }
    });
}
