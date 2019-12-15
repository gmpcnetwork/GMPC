pub use serde_json;
pub use libp2p::{Multiaddr, PeerId};
pub use libp2p::multiaddr;


use libp2p::core::ConnectedPoint;
use serde::{Deserialize, Serialize};
use slog_derive::SerdeValue;
use std::{collections::{HashMap, HashSet}, time::Duration};

/// this stores current network info
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, SerdeValue)]
pub struct NetWorkInfo {
    /// current peer server domain name
    pub peer_server_domain: String,

    /// the peers listening currently
    pub listening_addrs: HashSet<Multiaddr>,

    /// the peers current connected
    pub current_peers: HashMap<String, NetConn>,

    /// serlized peers
	pub peerset: serde_json::Value,
}

/// network connection struct
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, SerdeValue)]
pub struct NetConn {
    /// current connection version
    version: String,
    /// the node local address
    local_addr: Multiaddr,
    /// the address that receive data
    receive_addr: Multiaddr,
} 

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Endpoint {
	/// We are dialing the given address.
	Dialing(Multiaddr),
	/// We are listening.
	Listener {
		/// Local address of the connection.
		local_addr: Multiaddr,
		/// Address data is sent back to.
		receive_addr: Multiaddr,
	},
}

impl From<ConnectedPoint> for Endpoint {
	fn from(endpoint: ConnectedPoint) -> Self {
		match endpoint {
			ConnectedPoint::Dialer { address } => Endpoint::Dialing(address),
			ConnectedPoint::Listener { listen_addr, send_back_addr } =>
            Endpoint::Listener {
					local_addr:listen_addr,
					receive_addr: send_back_addr
				}
		}
	}
}
