#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use crate::codec::{Decode, Encode, Codec, Input, Output, HasCompact, EncodeAsRef, Error};
use crate::traits::{
	self, Member, SimpleArithmetic, SimpleBitOps, Hash as HashT,
	MaybeSerializeDeserialize, MaybeSerialize, MaybeDisplay,
};
use crate::generic::Digest;
use primitives::U256;
use rstd::{
	convert::TryFrom,
	fmt::Debug,
};

/// Abstraction over a block header for a substrate chain.
#[derive(PartialEq, Eq, Clone, primitives::RuntimeDebug)]
pub struct Header<Number: Copy + Into<U256> + TryFrom<U256>, Hash: HashT> {
	/// The parent hash.
	pub parent_hash: Hash::Output,
	/// The block number.
	#[cfg_attr(feature = "std", serde(
		serialize_with = "serialize_number",
		deserialize_with = "deserialize_number"))]
	pub number: Number,
	/// The state trie merkle root
	pub state_root: Hash::Output,
	/// The merkle root of the extrinsics.
	pub extrinsics_root: Hash::Output,
	/// A chain-specific digest of data useful for light clients or referencing auxiliary data.
	pub digest: Digest<Hash::Output>,
}

#[cfg(feature = "std")]
pub fn serialize_number<S, T: Copy + Into<U256> + TryFrom<U256>>(
	val: &T, s: S,
) -> Result<S::Ok, S::Error> where S: serde::Serializer {
	let u256: U256 = (*val).into();
	serde::Serialize::serialize(&u256, s)
}

#[cfg(feature = "std")]
pub fn deserialize_number<'a, D, T: Copy + Into<U256> + TryFrom<U256>>(
	d: D,
) -> Result<T, D::Error> where D: serde::Deserializer<'a> {
	let u256: U256 = serde::Deserialize::deserialize(d)?;
	TryFrom::try_from(u256).map_err(|_| serde::de::Error::custom("Try from failed"))
}

impl<Number, Hash> Decode for Header<Number, Hash> where
	Number: HasCompact + Copy + Into<U256> + TryFrom<U256>,
	Hash: HashT,
	Hash::Output: Decode,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		Ok(Header {
			parent_hash: Decode::decode(input)?,
			number: <<Number as HasCompact>::Type>::decode(input)?.into(),
			state_root: Decode::decode(input)?,
			extrinsics_root: Decode::decode(input)?,
			digest: Decode::decode(input)?,
		})
	}
}

impl<Number, Hash> Encode for Header<Number, Hash> where
	Number: HasCompact + Copy + Into<U256> + TryFrom<U256>,
	Hash: HashT,
	Hash::Output: Encode,
{
	fn encode_to<T: Output>(&self, dest: &mut T) {
		dest.push(&self.parent_hash);
		dest.push(&<<<Number as HasCompact>::Type as EncodeAsRef<_>>::RefType>::from(&self.number));
		dest.push(&self.state_root);
		dest.push(&self.extrinsics_root);
		dest.push(&self.digest);
	}
}

impl<Number, Hash> traits::Headerly for Header<Number, Hash> where
	Number: Member + MaybeSerializeDeserialize + Debug + rstd::hash::Hash + MaybeDisplay +
		SimpleArithmetic + Codec + Copy + Into<U256> + TryFrom<U256>,
	Hash: HashT,
	Hash::Output: Default + rstd::hash::Hash + Copy + Member +
		MaybeSerialize + Debug + MaybeDisplay + SimpleBitOps + Codec,
{
	type Number = Number;
	type Hash = <Hash as HashT>::Output;
	type Hashing = Hash;

	fn number(&self) -> &Self::Number { &self.number }
	fn set_number(&mut self, num: Self::Number) { self.number = num }

	fn extrinsics_root(&self) -> &Self::Hash { &self.extrinsics_root }
	fn set_extrinsics_root(&mut self, root: Self::Hash) { self.extrinsics_root = root }

	fn state_root(&self) -> &Self::Hash { &self.state_root }
	fn set_state_root(&mut self, root: Self::Hash) { self.state_root = root }

	fn parent_hash(&self) -> &Self::Hash { &self.parent_hash }
	fn set_parent_hash(&mut self, hash: Self::Hash) { self.parent_hash = hash }

	fn digest(&self) -> &Digest<Self::Hash> { &self.digest }

	fn digest_mut(&mut self) -> &mut Digest<Self::Hash> {
		#[cfg(feature = "std")]
		log::debug!(target: "header", "Retrieving mutable reference to digest");
		&mut self.digest
	}

	fn new(
		number: Self::Number,
		extrinsics_root: Self::Hash,
		state_root: Self::Hash,
		parent_hash: Self::Hash,
		digest: Digest<Self::Hash>,
	) -> Self {
		Header {
			number,
			extrinsics_root,
			state_root,
			parent_hash,
			digest,
		}
	}
}

impl<Number, Hash> Header<Number, Hash> where
	Number: Member + rstd::hash::Hash + Copy + MaybeDisplay + SimpleArithmetic + Codec + Into<U256> + TryFrom<U256>,
	Hash: HashT,
	Hash::Output: Default + rstd::hash::Hash + Copy + Member + MaybeDisplay + SimpleBitOps + Codec,
 {
	/// Convenience helper for computing the hash of the header without having
	/// to import the trait.
	pub fn hash(&self) -> Hash::Output {
		Hash::hash_of(self)
	}
}