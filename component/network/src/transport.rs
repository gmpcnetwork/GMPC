// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use futures::prelude::*;
use libp2p::{
	InboundUpgradeExt, OutboundUpgradeExt, PeerId, Transport,
	mplex, identity, secio, yamux, bandwidth, wasm_ext
};
#[cfg(not(target_os = "unknown"))]
use libp2p::{tcp, dns, websocket, noise};
#[cfg(not(target_os = "unknown"))]
use libp2p::core::{either::EitherError, either::EitherOutput};
use libp2p::core::{self, upgrade, transport::boxed::Boxed, transport::OptionalTransport, muxing::StreamMuxerBox};
use std::{io, sync::Arc, time::Duration, usize,fmt};
use std::error::Error as StdError;

use futures::{future, Future, IntoFuture};
use serde_json::Value as JsonValue;

pub use self::bandwidth::BandwidthSinks;
use super::error::{Error, Never};

/// Builds the transport that serves as a common ground for all connections.
///
/// If `memory_only` is true, then only communication within the same process are allowed. Only
/// addresses with the format `/memory/...` are allowed.
///
/// Returns a `BandwidthSinks` object that allows querying the average bandwidth produced by all
/// the connections spawned with this transport.
pub fn build_transport(
	keypair: identity::Keypair,
	memory_only: bool,
	wasm_external_transport: Option<wasm_ext::ExtTransport>
) -> (Boxed<(PeerId, StreamMuxerBox), io::Error>, Arc<bandwidth::BandwidthSinks>) {
	// Build configuration objects for encryption mechanisms.
	#[cfg(not(target_os = "unknown"))]
	let noise_config = {
		let noise_keypair = noise::Keypair::new().into_authentic(&keypair)
			// For more information about this panic, see in "On the Importance of Checking
			// Cryptographic Protocols for Faults" by Dan Boneh, Richard A. DeMillo,
			// and Richard J. Lipton.
			.expect("can only fail in case of a hardware bug; since this signing is performed only \
				once and at initialization, we're taking the bet that the inconvenience of a very \
				rare panic here is basically zero");
		noise::NoiseConfig::ix(noise_keypair)
	};
	let secio_config = secio::SecioConfig::new(keypair);

	// Build configuration objects for multiplexing mechanisms.
	let mut mplex_config = mplex::MplexConfig::new();
	mplex_config.max_buffer_len_behaviour(mplex::MaxBufferBehaviour::Block);
	mplex_config.max_buffer_len(usize::MAX);
	let yamux_config = yamux::Config::default();

	// Build the base layer of the transport.
	let transport = if let Some(t) = wasm_external_transport {
		OptionalTransport::some(t)
	} else {
		OptionalTransport::none()
	};
	#[cfg(not(target_os = "unknown"))]
	let transport = transport.or_transport(if !memory_only {
		let desktop_trans = tcp::TcpConfig::new();
		let desktop_trans = websocket::WsConfig::new(desktop_trans.clone())
			.or_transport(desktop_trans);
		OptionalTransport::some(dns::DnsConfig::new(desktop_trans))
	} else {
		OptionalTransport::none()
	});

	let transport = transport.or_transport(if memory_only {
		OptionalTransport::some(libp2p::core::transport::MemoryTransport::default())
	} else {
		OptionalTransport::none()
	});

	let (transport, sinks) = bandwidth::BandwidthLogging::new(transport, Duration::from_secs(5));

	// Encryption

	// For non-WASM, we support both secio and noise.
	#[cfg(not(target_os = "unknown"))]
	let transport = transport.and_then(move |stream, endpoint| {
		let upgrade = core::upgrade::SelectUpgrade::new(noise_config, secio_config);
		core::upgrade::apply(stream, upgrade, endpoint, upgrade::Version::V1)
			.and_then(|out| match out {
				// We negotiated noise
				EitherOutput::First((remote_id, out)) => {
					let remote_key = match remote_id {
						noise::RemoteIdentity::IdentityKey(key) => key,
						_ => return Err(upgrade::UpgradeError::Apply(EitherError::A(noise::NoiseError::InvalidKey)))
					};
					Ok((EitherOutput::First(out), remote_key.into_peer_id()))
				}
				// We negotiated secio
				EitherOutput::Second((remote_id, out)) =>
					Ok((EitherOutput::Second(out), remote_id))
			})
	});

	// For WASM, we only support secio for now.
	#[cfg(target_os = "unknown")]
	let transport = transport.and_then(move |stream, endpoint| {
		core::upgrade::apply(stream, secio_config, endpoint, upgrade::Version::V1)
			.and_then(|(id, stream)| Ok((stream, id)))
	});

	// Multiplexing
	let transport = transport.and_then(move |(stream, peer_id), endpoint| {
			let peer_id2 = peer_id.clone();
			let upgrade = core::upgrade::SelectUpgrade::new(yamux_config, mplex_config)
				.map_inbound(move |muxer| (peer_id, muxer))
				.map_outbound(move |muxer| (peer_id2, muxer));

			core::upgrade::apply(stream, upgrade, endpoint, upgrade::Version::V1)
				.map(|(id, muxer)| (id, core::muxing::StreamMuxerBox::new(muxer)))
		})

		.timeout(Duration::from_secs(20))
		.map_err(|err| io::Error::new(io::ErrorKind::Other, err))
		.boxed();

	(transport, sinks)
}



pub type Result = Box<Future<Item = Option<JsonValue>, Error = Error> + Send>;

pub trait Dispatch {
	type Error: Into<Box<StdError + Send + Sync>>;
	type Future: Future<Item = Option<JsonValue>, Error = Self::Error>;

	fn call(&mut self, req: JsonValue) -> Self::Future;
}

pub struct DispatchFn<F> {
	f: F,
}

pub fn dispatch_fn<F, D>(f: F) -> DispatchFn<F>
	where
		F: Fn(JsonValue) -> D,
		D: IntoFuture, {
	DispatchFn {
		f,
	}
}

impl<F, Ret> Dispatch for DispatchFn<F>
	where
		F: Fn(JsonValue) -> Ret,
		Ret: IntoFuture<Item = Option<JsonValue>>,
		Ret::Error: Into<Box<StdError + Send + Sync>>,
{
	type Error = Ret::Error;
	type Future = Ret::Future;

	fn call(&mut self, req: JsonValue) -> Self::Future {
		(self.f)(req).into_future()
	}
}

impl<F> IntoFuture for DispatchFn<F> {
	type Future = future::FutureResult<Self::Item, Self::Error>;
	type Item = Self;
	type Error = Never;

	fn into_future(self) -> Self::Future {
		future::ok(self)
	}
}

impl<F> fmt::Debug for DispatchFn<F> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("impl Dispatch").finish()
	}
}

/// An asynchronous constructor of `Dispatch`s.
pub trait NewDispatch {
	type Error: Into<Box<StdError + Send + Sync>>;
	type Dispatch: Dispatch<Error = Self::Error>;
	type Future: Future<Item = Self::Dispatch, Error = Self::InitError>;
	type InitError: Into<Box<StdError + Send + Sync>>;

	/// Create a new `Dispatch`.
	fn new_dispatch(&self) -> Self::Future;
}

impl<F, R, D> NewDispatch for F
	where
		F: Fn() -> R,
		R: IntoFuture<Item = D>,
		R::Error: Into<Box<StdError + Send + Sync>>,
		D: Dispatch,
{
	type Error = D::Error;
	type Dispatch = D;
	type Future = R::Future;
	type InitError = R::Error;

	fn new_dispatch(&self) -> Self::Future {
		(*self)().into_future()
	}
}
