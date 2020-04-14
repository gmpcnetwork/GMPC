
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Weak};

use hash_fetch::fetch::Client as FetchClient;
use jsonrpc_core::{self as core, MetaIoHandler};
use light::client::LightChainClient;
use light::{Cache as LightDataCache, TransactionQueue as LightTransactionQueue};
use miner::external::ExternalMiner;

use parking_lot::{Mutex, RwLock};
use sync::{LightSync, ManageNetwork, SyncProvider};
use updater::Updater;

/// Light client notifier. Doesn't do anything yet, but might in the future.
pub struct LightClientNotifier;

impl ActivityNotifier for LightClientNotifier {
	fn active(&self) {}
}

/// RPC dependencies for a light client.
pub struct LightDependencies<T> {
	pub signer_service: Arc<SignerService>,
	pub client: Arc<T>,
	pub sync: Arc<LightSync>,
	pub net: Arc<dyn ManageNetwork>,
	pub accounts: Arc<AccountProvider>,
	pub logger: Arc<RotatingLogger>,
	pub settings: Arc<NetworkSettings>,
	pub on_demand: Arc<::light::on_demand::OnDemand>,
	pub cache: Arc<Mutex<LightDataCache>>,
	pub transaction_queue: Arc<RwLock<LightTransactionQueue>>,
	pub ws_address: Option<Host>,
	pub fetch: FetchClient,
	pub geth_compatibility: bool,
	pub experimental_rpcs: bool,
	pub executor: Executor,
	pub private_tx_service: Option<Arc<PrivateTransactionManager>>,
	pub gas_price_percentile: usize,
	pub poll_lifetime: u32,
}

impl<C: LightChainClient + 'static> LightDependencies<C> {
	fn extend_api<T: core::Middleware<Metadata>>(
		&self,
		handler: &mut MetaIoHandler<Metadata, T>,
		apis: &HashSet<Api>,
		for_generic_pubsub: bool,
	) {
		use gmpc_rpc::v1::*;

		let dispatcher = LightDispatcher::new(
			self.sync.clone(),
			self.client.clone(),
			self.on_demand.clone(),
			self.cache.clone(),
			self.transaction_queue.clone(),
			Arc::new(Mutex::new(dispatch::Reservations::new(
				self.executor.clone(),
			))),
			self.gas_price_percentile,
		);
		let account_signer = Arc::new(dispatch::Signer::new(self.accounts.clone())) as _;
		let accounts = account_utils::accounts_list(self.accounts.clone());

		for api in apis {
			match *api {
				Api::Debug => {
					warn!(target: "rpc", "Debug API is not available in light client mode.")
				}
				Api::Web3 => {
					handler.extend_with(Web3Client::default().to_delegate());
				}
				Api::Net => {
					handler.extend_with(light::NetClient::new(self.sync.clone()).to_delegate());
				}
				Api::Eth => {
					let client = light::EthClient::new(
						self.sync.clone(),
						self.client.clone(),
						self.on_demand.clone(),
						self.transaction_queue.clone(),
						accounts.clone(),
						self.cache.clone(),
						self.gas_price_percentile,
						self.poll_lifetime,
					);
					handler.extend_with(Eth::to_delegate(client.clone()));

					if !for_generic_pubsub {
						handler.extend_with(EthFilter::to_delegate(client));
						add_signing_methods!(EthSigning, handler, self, (&dispatcher, &account_signer));
					}
				}
				Api::EthPubSub => {
					let receiver = self.transaction_queue.write().pending_transactions_receiver();

					let mut client = EthPubSubClient::light(
						self.client.clone(),
						self.on_demand.clone(),
						self.sync.clone(),
						self.cache.clone(),
						self.executor.clone(),
						self.gas_price_percentile,
						receiver
					);

					let weak_client = Arc::downgrade(&self.client);

					client.add_sync_notifier(self.sync.sync_notification(), move |state| {
						let client = weak_client.upgrade()?;
						let queue_info = client.queue_info();

						let is_syncing_state = match state { SyncState::Idle | SyncState::NewBlocks => false, _ => true };
						let is_verifying = queue_info.unverified_queue_size + queue_info.verified_queue_size > 3;

						Some(PubSubSyncStatus {
							syncing: is_verifying || is_syncing_state,
						})
					});

					self.client.add_listener(client.handler() as Weak<_>);
					handler.extend_with(EthPubSub::to_delegate(client));
				}
				Api::GmpcTransactionsPool => {
					if !for_generic_pubsub {
						let receiver = self.transaction_queue.write().full_transactions_receiver();
						let client = TransactionsPoolClient::new(self.executor.clone(), receiver);
						handler.extend_with(TransactionsPoolClient::to_delegate(client));
					}
				}
				Api::Personal => {
					#[cfg(feature = "accounts")]
					handler.extend_with(
						PersonalClient::new(
							&self.accounts,
							dispatcher.clone(),
							self.geth_compatibility,
							self.experimental_rpcs,
						).to_delegate(),
					);
				}
				Api::Signer => {
					handler.extend_with(
						SignerClient::new(
							account_signer.clone(),
							dispatcher.clone(),
							&self.signer_service,
							self.executor.clone(),
						).to_delegate(),
					);
				}
				Api::Gmpc => {
					let signer = match self.signer_service.is_enabled() {
						true => Some(self.signer_service.clone()),
						false => None,
					};
					handler.extend_with(
						light::GmpcClient::new(
							Arc::new(dispatcher.clone()),
							self.logger.clone(),
							self.settings.clone(),
							signer,
							self.ws_address.clone(),
							self.gas_price_percentile,
						).to_delegate(),
					);
					#[cfg(feature = "accounts")]
					handler.extend_with(
						GmpcAccountsInfo::to_delegate(GmpcAccountsClient::new(&self.accounts))
					);

					if !for_generic_pubsub {
						add_signing_methods!(GmpcSigning, handler, self, (&dispatcher, &account_signer));
					}
				}
				Api::GmpcPubSub => {
					if !for_generic_pubsub {
						let mut rpc = MetaIoHandler::default();
						let apis = ApiSet::List(apis.clone())
							.retain(ApiSet::PubSub)
							.list_apis();
						self.extend_api(&mut rpc, &apis, true);
						handler.extend_with(
							PubSubClient::new(rpc, self.executor.clone()).to_delegate(),
						);
					}
				}
				Api::GmpcAccounts => {
					#[cfg(feature = "accounts")]
					handler.extend_with(GmpcAccounts::to_delegate(GmpcAccountsClient::new(&self.accounts)));
				}
				Api::GmpcSet => handler.extend_with(
					light::GmpcSetClient::new(self.client.clone(), self.sync.clone(), self.fetch.clone())
						.to_delegate(),
				),
				Api::Traces => handler.extend_with(light::TracesClient.to_delegate()),
				Api::Rpc => {
					let modules = to_modules(&apis);
					handler.extend_with(RpcClient::new(modules).to_delegate());
				}
				Api::SecretStore => {
					#[cfg(feature = "accounts")]
					handler.extend_with(SecretStoreClient::new(&self.accounts).to_delegate());
				}
				Api::Private => {
					if let Some(ref tx_manager) = self.private_tx_service {
						let private_tx_service = Some(tx_manager.clone());
						handler.extend_with(PrivateClient::new(private_tx_service).to_delegate());
					}
				}
				Api::Deprecated => {},
			}
		}
	}
}

impl<T: LightChainClient + 'static> Dependencies for LightDependencies<T> {
	type Notifier = LightClientNotifier;

	fn activity_notifier(&self) -> Self::Notifier {
		LightClientNotifier
	}

	fn extend_with_set<S>(&self, handler: &mut MetaIoHandler<Metadata, S>, apis: &HashSet<Api>)
	where
		S: core::Middleware<Metadata>,
	{
		self.extend_api(handler, apis, false)
	}
}
