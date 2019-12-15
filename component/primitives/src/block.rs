pub use traits::Blockly;
pub use trais::Headerly;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Block<Header, BlockBody> {
    pub header: Header,
    pub body: BlockBody
}

impl Blockly for Block<Header, BlockBody>{

}

