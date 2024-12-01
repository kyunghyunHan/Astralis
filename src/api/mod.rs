use serde::Deserialize;
pub mod account;
pub mod binance;
pub mod excution;
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceTrade {
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub transaction_time: i64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}
#[derive(Debug, Deserialize, Clone)]
pub struct FuturesAccountInfo {
    #[serde(rename = "feeTier")]
    pub fee_tier: i32,
    #[serde(rename = "canTrade")]
    pub can_trade: bool,
    #[serde(rename = "canDeposit")]
    pub can_deposit: bool,
    #[serde(rename = "canWithdraw")]
    pub can_withdraw: bool,
    #[serde(rename = "updateTime")]
    pub update_time: i64,
    #[serde(rename = "totalInitialMargin")]
    pub total_initial_margin: String,
    #[serde(rename = "totalMaintMargin")]
    pub total_maint_margin: String,
    #[serde(rename = "totalWalletBalance")]
    pub total_wallet_balance: String,
    #[serde(rename = "totalUnrealizedProfit")]
    pub total_unrealized_profit: String,
    #[serde(rename = "totalMarginBalance")]
    pub total_margin_balance: String,
    #[serde(rename = "totalPositionInitialMargin")]
    pub total_position_initial_margin: String,
    #[serde(rename = "totalOpenOrderInitialMargin")]
    pub total_open_order_initial_margin: String,
    #[serde(rename = "totalCrossWalletBalance")]
    pub total_cross_wallet_balance: String,
    #[serde(rename = "totalCrossUnPnl")]
    pub total_cross_un_pnl: String,
    #[serde(rename = "availableBalance")]
    pub available_balance: String,
    #[serde(rename = "maxWithdrawAmount")]
    pub max_withdraw_amount: String,
    pub assets: Vec<FuturesAsset>,
    pub positions: Vec<FuturesPosition>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct FuturesAsset {
    pub asset: String,
    #[serde(rename = "walletBalance")]
    pub wallet_balance: String,
    #[serde(rename = "unrealizedProfit")]
    pub unrealized_profit: String,
    #[serde(rename = "marginBalance")]
    pub margin_balance: String,
    #[serde(rename = "maintMargin")]
    pub maint_margin: String,
    #[serde(rename = "initialMargin")]
    pub initial_margin: String,
    #[serde(rename = "positionInitialMargin")]
    pub position_initial_margin: String,
    #[serde(rename = "openOrderInitialMargin")]
    pub open_order_initial_margin: String,
    #[serde(rename = "maxWithdrawAmount")]
    pub max_withdraw_amount: String,
    #[serde(rename = "crossWalletBalance")]
    pub cross_wallet_balance: String,
    #[serde(rename = "crossUnPnl")]
    pub cross_un_pnl: String,
    #[serde(rename = "availableBalance")]
    pub available_balance: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct FuturesPosition {
    pub symbol: String,
    #[serde(rename = "initialMargin")]
    pub initial_margin: String,
    #[serde(rename = "maintMargin")]
    pub maint_margin: String,
    #[serde(rename = "unrealizedProfit")]
    pub unrealized_profit: String,
    #[serde(rename = "positionInitialMargin")]
    pub position_initial_margin: String,
    pub leverage: String,
    pub isolated: bool,
    #[serde(rename = "entryPrice")]
    pub entry_price: String,
    #[serde(rename = "maxNotional")]
    pub max_notional: String,
    #[serde(rename = "positionSide")]
    pub position_side: String,
    #[serde(rename = "positionAmt")]
    pub position_amt: String,
}
#[derive(Debug, Deserialize, Clone)]
struct BinanceCandle {
    open_time: u64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    close_time: u64,
    quote_asset_volume: String,
    number_of_trades: u32,
    taker_buy_base_asset_volume: String,
    taker_buy_quote_asset_volume: String,
    ignore: String,
}
