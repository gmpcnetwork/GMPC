mod wal;

use log::{info,trace,warn};
use serde::{Deserialize, Serialize};

use wal::{WAL};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeoutStruct {
    pub height: u64,
    pub round:  u64,
    pub duration: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    timeout: TimeoutStruct,
    wal: WAL, // write ahead log file
}

impl State {
    pub fn print_info(&self) {
        trace!("current state info {:?}", self);
    }

}

