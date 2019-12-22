mod state;
mod config;

use state::{State};
use config::{GBFTConfig};

pub struct GPBFInitlizer {
    pub state: State,
    pub current: u64,
    pub config: GBFTConfig,
}

