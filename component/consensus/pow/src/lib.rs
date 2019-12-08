mod keccak;
use keccak::{keccak_512, keccak_256, H256};

use std::mem;

pub const FNV_PRIME: u32 = 0x01000193;

fn fnv_hash(x: u32, y: u32) -> u32 {
	return x.wrapping_mul(FNV_PRIME) ^ y;
}

/// Difficulty quick check for POW preverification
///
/// `header_hash`      The hash of the header
/// `nonce`            The block's nonce
/// `mix_hash`         The mix digest hash
/// Boundary recovered from mix hash
pub fn calc_difficulty(header_hash: &H256, nonce: u64, mix_hash: &H256) -> H256 {
	unsafe {
			let mut buf = [0u8; 64 + 32];

			let hash_len = header_hash.len();
			buf[..hash_len].copy_from_slice(header_hash);
			buf[hash_len..hash_len + mem::size_of::<u64>()].copy_from_slice(&nonce.to_ne_bytes());

			keccak_512::unchecked(buf.as_mut_ptr(), 64, buf.as_ptr(), 40);
			buf[64..].copy_from_slice(mix_hash);

			let mut hash = [0u8; 32];
			keccak_256::unchecked(hash.as_mut_ptr(), hash.len(), buf.as_ptr(), buf.len());

			hash
		}
}



	#[test]
	fn test_calc_difficulty() {
		let hash = [
			0xf5, 0x7e, 0x6f, 0x3a, 0xcf, 0xc0, 0xdd, 0x4b, 0x5b, 0xf2, 0xbe, 0xe4, 0x0a, 0xb3,
			0x35, 0x8a, 0xa6, 0x87, 0x73, 0xa8, 0xd0, 0x9f, 0x5e, 0x59, 0x5e, 0xab, 0x55, 0x94,
			0x05, 0x52, 0x7d, 0x72,
		];
		let mix_hash = [
			0x1f, 0xff, 0x04, 0xce, 0xc9, 0x41, 0x73, 0xfd, 0x59, 0x1e, 0x3d, 0x89, 0x60, 0xce,
			0x6b, 0xdf, 0x8b, 0x19, 0x71, 0x04, 0x8c, 0x71, 0xff, 0x93, 0x7b, 0xb2, 0xd3, 0x2a,
			0x64, 0x31, 0xab, 0x6d,
		];
		let nonce = 0xd7b3ac70a301a249;
		let boundary_good = [
			0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x3e, 0x9b, 0x6c, 0x69, 0xbc, 0x2c, 0xe2, 0xa2,
			0x4a, 0x8e, 0x95, 0x69, 0xef, 0xc7, 0xd7, 0x1b, 0x33, 0x35, 0xdf, 0x36, 0x8c, 0x9a,
			0xe9, 0x7e, 0x53, 0x84,
		];
		assert_eq!(calc_difficulty(&hash, nonce, &mix_hash, false)[..], boundary_good[..]);
		let boundary_bad = [
			0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x3a, 0x9b, 0x6c, 0x69, 0xbc, 0x2c, 0xe2, 0xa2,
			0x4a, 0x8e, 0x95, 0x69, 0xef, 0xc7, 0xd7, 0x1b, 0x33, 0x35, 0xdf, 0x36, 0x8c, 0x9a,
			0xe9, 0x7e, 0x53, 0x84,
		];
		assert!(calc_difficulty(&hash, nonce, &mix_hash, false)[..] != boundary_bad[..]);
	}
