use nix::sys::signal::Signal;
use signal_hook::iterator::Signals;

use crate::events::{Event, EventSender};

pub fn start(signals: Signals, sender: EventSender) {
    loop {
        for signal in signals.pending() {
            let signal = Signal::from_c_int(signal).expect("invalid signal received");
            if sender.send(Event::Signal(signal)).is_err() {
                return;
            }
        }
    }
}
