//! GMPC network possible errors.

use libp2p::{PeerId, Multiaddr};

use std::fmt;

/// Result type alias for the network.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for the network.
#[derive(derive_more::Display, derive_more::From)]
pub enum Error {
	/// Io error
	Io(std::io::Error),
	/// Client error
	Client(sp_blockchain::Error),
	/// The same bootnode (based on address) is registered with two different peer ids.
	#[display(
		fmt = "The same bootnode (`{}`) is registered with two different peer ids: `{}` and `{}`",
		address,
		first_id,
		second_id,
	)]
	DuplicateBootnode {
		/// The address of the bootnode.
		address: Multiaddr,
		/// The first peer id that was found for the bootnode.
		first_id: PeerId,
		/// The second peer id that was found for the bootnode.
		second_id: PeerId,
	},
	/// Prometheus metrics error.
	Prometheus(prometheus_endpoint::PrometheusError)
}

// Make `Debug` use the `Display` implementation.
impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, f)
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Io(ref err) => Some(err),
			Error::Client(ref err) => Some(err),
			Error::DuplicateBootnode { .. } => None,
			Error::Prometheus(ref err) => Some(err),
		}
	}
}
