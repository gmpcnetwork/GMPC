
pub trait Client {
    fn(&self) -> ClientInfo<Block>;
}