//! Hasher implementation for the Keccak-256 hash

#![warn(missing_docs)]

use hash_db::Hasher;
use ethereum_types::H256;
use tiny_keccak::Keccak;
use plain_hasher::PlainHasher;

/// Concrete `Hasher` impl for the Keccak-256 hash
#[derive(Default, Debug, Clone, PartialEq)]
pub struct KeccakHasher;
impl Hasher for KeccakHasher {
	type Out = H256;
	type StdHasher = PlainHasher;
	const LENGTH: usize = 32;
	fn hash(x: &[u8]) -> Self::Out {
		let mut out = [0; 32];
		Keccak::keccak256(x, &mut out);
		out.into()
	}
}
