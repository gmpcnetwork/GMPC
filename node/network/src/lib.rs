
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

mod behaviour;
mod chain;
mod debug_info;
mod discovery;
mod on_demand_layer;
mod protocol;
mod service;
mod transport;
mod utils;

pub mod config;
pub mod error;
pub mod network_state;

pub use service::{NetworkService, NetworkStateInfo, NetworkWorker, ExHashT, ReportHandle};
pub use protocol::PeerInfo;
pub use protocol::event::{Event, DhtEvent, ObservedRole};
pub use protocol::sync::SyncState;
pub use libp2p::{Multiaddr, PeerId};
#[doc(inline)]
pub use libp2p::multiaddr;

pub use sc_peerset::ReputationChange;

/// The maximum allowed number of established connections per peer.
///
/// Typically, and by design of the network behaviours in this crate,
/// there is a single established connection per peer. However, to
/// avoid unnecessary and nondeterministic connection closure in
/// case of (possibly repeated) simultaneous dialing attempts between
/// two peers, the per-peer connection limit is not set to 1 but 2.
const MAX_CONNECTIONS_PER_PEER: usize = 2;
