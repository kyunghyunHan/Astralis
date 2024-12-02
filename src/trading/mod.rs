pub mod markey_order;

#[derive(Debug, Clone, Copy)]
pub enum TradeType {
    Buy,
    Sell,
}