
//! Blockchain access trait

use sp_blockchain::{Error, HeaderBackend, HeaderMetadata};
use sc_client_api::{BlockBackend, ProofProvider};
use sp_runtime::traits::{Block as BlockT, BlockIdTo};

/// Local client abstraction for the network.
pub trait Client<Block: BlockT>: HeaderBackend<Block> + ProofProvider<Block> + BlockIdTo<Block, Error = Error>
	+ BlockBackend<Block> + HeaderMetadata<Block, Error = Error> + Send + Sync
{}

impl<Block: BlockT, T> Client<Block> for T
	where
		T: HeaderBackend<Block> + ProofProvider<Block> + BlockIdTo<Block, Error = Error>
			+ BlockBackend<Block> + HeaderMetadata<Block, Error = Error> + Send + Sync
{}

/// Finality proof provider.
pub trait FinalityProofProvider<Block: BlockT>: Send + Sync {
	/// Prove finality of the block.
	fn prove_finality(&self, for_block: Block::Hash, request: &[u8]) -> Result<Option<Vec<u8>>, Error>;
}

impl<Block: BlockT> FinalityProofProvider<Block> for () {
	fn prove_finality(&self, _for_block: Block::Hash, _request: &[u8]) -> Result<Option<Vec<u8>>, Error> {
		Ok(None)
	}
}
