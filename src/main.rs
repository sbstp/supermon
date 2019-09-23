mod events;
mod reactor;
mod spec;

use serde_yaml;
use std::fs::File;
use std::io::BufReader;

use crate::spec::Spec;

fn main() {
    let spec_path = std::env::args_os().nth(1).expect("first argument must be spec path");
    let file = File::open(spec_path).expect("unable to open spec for reading");
    let reader = BufReader::new(file);
    let spec: Spec = serde_yaml::from_reader(reader).expect("invalid spec");

    reactor::run(spec);
}
