/// Abstraction around hashing
// Stupid bug in the Rust compiler believes derived
// traits must be fulfilled by all type parameters.
pub trait Hash: 'static + MaybeSerializeDeserialize + Debug + Clone + Eq + PartialEq {
	/// The hash type produced.
	type Output: Member + MaybeSerializeDeserialize + Debug + rstd::hash::Hash
		+ AsRef<[u8]> + AsMut<[u8]> + Copy + Default + Encode + Decode;

	/// The associated hash_db Hasher type.
	type Hasher: Hasher<Out=Self::Output>;

	/// Produce the hash of some byte-slice.
	fn hash(s: &[u8]) -> Self::Output;

	/// Produce the hash of some codec-encodable value.
	fn hash_of<S: Encode>(s: &S) -> Self::Output {
		Encode::using_encoded(s, Self::hash)
	}

	/// The ordered Patricia tree root of the given `input`.
	fn ordered_trie_root(input: Vec<Vec<u8>>) -> Self::Output;

	/// The Patricia tree root of the given mapping.
	fn trie_root(input: Vec<(Vec<u8>, Vec<u8>)>) -> Self::Output;
}

pub trait Headerly: Clone + Eq + MaybeSerialize + Debug + 'static {
    type Number: Member + MaybeSerializeDeserialize + Debug + MaybeDisplay + Copy Codec;
    type Hash: Member + MaybeSerializeDeserialize + Debug + rstd::hash::Hash
    + Copy + MaybeDisplay + Default + SimpleBitOps + Codec + AsRef<[u8]> + AsMut<[u8]>;
    type Hashing: Hash<Output = Self::Hash>;
    
    fn numbers(&self)  -> &Self::Number;
    fn set_numer(&mut self, number: Self::Number);

    /// Returns a reference to the transaction root.
	fn tx_root(&self) -> &Self::Hash;
	/// Sets the transaction root.
	fn set_tx_root(&mut self, root: Self::Hash);

	/// Returns a reference to the state root.
	fn state_root(&self) -> &Self::Hash;
	/// Sets the state root.
	fn set_state_root(&mut self, root: Self::Hash);

	/// Returns a reference to the parent hash.
	fn parent_hash(&self) -> &Self::Hash;
	/// Sets the parent hash.
	fn set_parent_hash(&mut self, hash: Self::Hash);

	/// Returns a reference to the digest.
	fn digest(&self) -> &Digest<Self::Hash>;
	/// Get a mutable reference to the digest.
	fn digest_mut(&mut self) -> &mut Digest<Self::Hash>;

	/// Returns the hash of the header.
	fn hash(&self) -> Self::Hash {
		<Self::Hashing as Hash>::hash_of(self)
    }
    
}

pub trait Blockly: Clone + Eq + MaybeSerialize + Debug + 'static {
    /// Returns a reference to the header.
	fn header(&self) -> &Self::Header;
	/// Returns a reference to the list of extrinsics.
	fn extrinsics(&self) -> &[Self::Extrinsic];
	/// Split the block into header and list of extrinsics.
	fn deconstruct(self) -> (Self::Header, Vec<Self::Extrinsic>);
	/// Creates new block from header and extrinsics.
	fn new(header: Self::Header, extrinsics: Vec<Self::Extrinsic>) -> Self;
	/// Returns the hash of the block.
	fn hash(&self) -> Self::Hash {
		<<Self::Header as Header>::Hashing as Hash>::hash_of(self.header())
	}
	/// Create an encoded block from the given `header` and `extrinsics` without requiring to create an instance.
	fn encode_from(header: &Self::Header, extrinsics: &[Self::Extrinsic]) -> Vec<u8>;

}



#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Block<Header, BlockBody> {
    pub header: Header,
    pub body: BlockBody
}

