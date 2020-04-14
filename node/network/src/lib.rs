
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

pub use chain::{Client as ClientHandle, FinalityProofProvider};
pub use service::{
	NetworkService, NetworkWorker, TransactionPool, ExHashT, ReportHandle,
	NetworkStateInfo,
};
pub use protocol::{PeerInfo, Context, ProtocolConfig, message, specialization};
pub use protocol::event::{Event, DhtEvent};
pub use protocol::sync::SyncState;
pub use libp2p::{Multiaddr, PeerId};
#[doc(inline)]
pub use libp2p::multiaddr;

pub use message::{generic as generic_message, RequestId, Status as StatusMessage};
pub use on_demand_layer::{OnDemand, RemoteResponse};
pub use sc_peerset::ReputationChange;

// Used by the `construct_simple_protocol!` macro.
#[doc(hidden)]
pub use sp_runtime::traits::Block as BlockT;

use libp2p::core::ConnectedPoint;
use serde::{Deserialize, Serialize};
use slog_derive::SerdeValue;
use std::{collections::{HashMap, HashSet}, time::Duration};

/// Extension trait for `NetworkBehaviour` that also accepts discovering nodes.
pub trait DiscoveryNetBehaviour {
	/// Notify the protocol that we have learned about the existence of nodes.
	///
	/// Can (or most likely will) be called multiple times with the same `PeerId`s.
	///
	/// Also note that there is no notification for expired nodes. The implementer must add a TTL
	/// system, or remove nodes that will fail to reach.
	fn add_discovered_nodes(&mut self, nodes: impl Iterator<Item = PeerId>);
}

/// Returns general information about the networking.
///
/// Meant for general diagnostic purposes.
///
/// **Warning**: This API is not stable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, SerdeValue)]
#[serde(rename_all = "camelCase")]
pub struct NetworkState {
	/// PeerId of the local node.
	pub peer_id: String,
	/// List of addresses the node is currently listening on.
	pub listened_addresses: HashSet<Multiaddr>,
	/// List of addresses the node knows it can be reached as.
	pub external_addresses: HashSet<Multiaddr>,
	/// List of node we're connected to.
	pub connected_peers: HashMap<String, NetworkStatePeer>,
	/// List of node that we know of but that we're not connected to.
	pub not_connected_peers: HashMap<String, NetworkStateNotConnectedPeer>,
	/// Downloaded bytes per second averaged over the past few seconds.
	pub average_download_per_sec: u64,
	/// Uploaded bytes per second averaged over the past few seconds.
	pub average_upload_per_sec: u64,
	/// State of the peerset manager.
	pub peerset: serde_json::Value,
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatePeer {
	/// How we are connected to the node.
	pub endpoint: NetworkStatePeerEndpoint,
	/// Node information, as provided by the node itself. Can be empty if not known yet.
	pub version_string: Option<String>,
	/// Latest ping duration with this node.
	pub latest_ping_time: Option<Duration>,
	/// If true, the peer is "enabled", which means that we try to open Substrate-related protocols
	/// with this peer. If false, we stick to Kademlia and/or other network-only protocols.
	pub enabled: bool,
	/// If true, the peer is "open", which means that we have a Substrate-related protocol
	/// with this peer.
	pub open: bool,
	/// List of addresses known for this node.
	pub known_addresses: HashSet<Multiaddr>,
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStateNotConnectedPeer {
	/// List of addresses known for this node.
	pub known_addresses: HashSet<Multiaddr>,
	/// Node information, as provided by the node itself, if we were ever connected to this node.
	pub version_string: Option<String>,
	/// Latest ping duration with this node, if we were ever connected to this node.
	pub latest_ping_time: Option<Duration>,
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkStatePeerEndpoint {
	/// We are dialing the given address.
	Dialing(Multiaddr),
	/// We are listening.
	Listening {
		/// Local address of the connection.
		local_addr: Multiaddr,
		/// Address data is sent back to.
		send_back_addr: Multiaddr,
	},
}

impl From<ConnectedPoint> for NetworkStatePeerEndpoint {
	fn from(endpoint: ConnectedPoint) -> Self {
		match endpoint {
			ConnectedPoint::Dialer { address } =>
				NetworkStatePeerEndpoint::Dialing(address),
			ConnectedPoint::Listener { local_addr, send_back_addr } =>
				NetworkStatePeerEndpoint::Listening {
					local_addr,
					send_back_addr
				}
		}
	}
}
