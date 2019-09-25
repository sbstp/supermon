mod events;
mod reactor;
mod reaper;
mod spec;
mod signals;
mod utils;

use std::fs::File;
use std::io::BufReader;
use std::thread;

use serde_yaml;
use crossbeam_channel::bounded;
use signal_hook::iterator::Signals;

use crate::spec::Spec;
use crate::events::{EventSender, EventReceiver};

fn main() {
    let spec_path = std::env::args_os().nth(1).expect("first argument must be spec path");
    let file = File::open(spec_path).expect("unable to open spec for reading");
    let reader = BufReader::new(file);
    let spec: Spec = serde_yaml::from_reader(reader).expect("invalid spec");

    let signals = Signals::new(&[
        signal_hook::SIGTERM,
        signal_hook::SIGINT,
    ]).expect("unable to register signal handlers");

    let (sender, receiver): (EventSender, EventReceiver) = bounded(128);

    let signal_sender = sender.clone();
    let reaper_sender = sender.clone();

    thread::spawn(move || {
        signals::start(signals, signal_sender);
    });

    thread::spawn(move || {
        reaper::start(reaper_sender);
    });



    reactor::run(spec, sender, receiver);
}
