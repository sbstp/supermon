use nix::sys::wait::{wait, WaitStatus};

use crate::events::{Event, EventSender};

pub fn start(sender: EventSender) {
    loop {
        if let Ok(status) = wait() {
            match status {
                WaitStatus::Exited(pid, code) => {
                    if sender.send(Event::Exited(pid.into(), code)).is_err() {
                        return;
                    }
                }
                WaitStatus::Signaled(pid, signal, _) => {
                    if sender.send(Event::Signaled(pid.into(), signal)).is_err() {
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}
