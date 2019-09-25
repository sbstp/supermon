use std::fmt;

use nix::unistd;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Pid(pub i64);

impl Pid {
    pub fn to_nix(&self) -> unistd::Pid {
        unistd::Pid::from_raw(self.0 as i32)
    }
}

impl From<u32> for Pid {
    fn from(pid: u32) -> Pid {
        Pid(pid as i64)
    }
}

impl From<unistd::Pid> for Pid {
    fn from(pid: unistd::Pid) -> Pid {
        Pid(pid.as_raw() as i64)
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
