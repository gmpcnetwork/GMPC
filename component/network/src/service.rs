/// Get network state.
///
/// everywhere about this. Please don't use this function to retrieve actual information.
pub fn network_state(&mut self) -> NetworkState {
	let swarm = &mut self.network_service;
	let open = swarm.user_protocol().open_peers().cloned().collect::<Vec<_>>();

	let connected_peers = {
		let swarm = &mut *swarm;
		open.iter().filter_map(move |peer_id| {
			let known_addresses = NetworkBehaviour::addresses_of_peer(&mut **swarm, peer_id)
				.into_iter().collect();

			let endpoint = if let Some(e) = swarm.node(peer_id).map(|i| i.endpoint()) {
				e.clone().into()
			} else {
				error!(target: "sub-libp2p", "Found state inconsistency between custom protocol \
					and debug information about {:?}", peer_id);
				return None
			};

			Some((peer_id.to_base58(), NetworkStatePeer {
				endpoint,
				version_string: swarm.node(peer_id)
					.and_then(|i| i.client_version().map(|s| s.to_owned())).clone(),
				latest_ping_time: swarm.node(peer_id).and_then(|i| i.latest_ping()),
				enabled: swarm.user_protocol().is_enabled(&peer_id),
				open: swarm.user_protocol().is_open(&peer_id),
				known_addresses,
			}))
		}).collect()
	};

	let not_connected_peers = {
		let swarm = &mut *swarm;
		let list = swarm.known_peers().filter(|p| open.iter().all(|n| n != *p))
			.cloned().collect::<Vec<_>>();
		list.into_iter().map(move |peer_id| {
			(peer_id.to_base58(), NetworkStateNotConnectedPeer {
				version_string: swarm.node(&peer_id)
					.and_then(|i| i.client_version().map(|s| s.to_owned())).clone(),
				latest_ping_time: swarm.node(&peer_id).and_then(|i| i.latest_ping()),
				known_addresses: NetworkBehaviour::addresses_of_peer(&mut **swarm, &peer_id)
					.into_iter().collect(),
			})
		}).collect()
	};

	NetworkState {
		peer_id: Swarm::<B, S, H>::local_peer_id(&swarm).to_base58(),
		listened_addresses: Swarm::<B, S, H>::listeners(&swarm).cloned().collect(),
		external_addresses: Swarm::<B, S, H>::external_addresses(&swarm).cloned().collect(),
		average_download_per_sec: self.service.bandwidth.average_download_per_sec(),
		average_upload_per_sec: self.service.bandwidth.average_upload_per_sec(),
		connected_peers,
		not_connected_peers,
		peerset: swarm.user_protocol_mut().peerset_debug_info(),
	}
}