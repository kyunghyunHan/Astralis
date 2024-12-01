#![allow(non_utf8_strings)]
use async_stream::stream;
use dotenv::dotenv;
use futures_util::SinkExt;
use futures_util::Stream; // Add this at the top with other imports
mod api;
mod models;
mod ui;
mod utils;
use api::FuturesAccountInfo;
use api::{
    account::binance_account_connection,
    binance::{binance_connection, fetch_candles, fetch_candles_async},
    excution::execute_trade,
    BinanceTrade,
};
use iced::futures::{channel::mpsc, StreamExt};
use iced::time::{self, Duration, Instant};
use iced::widget::Row;
use iced::Length::FillPortion;
use iced::{
    mouse,
    widget::{
        button, canvas,
        canvas::{
            event::{self, Event},
            Canvas, Program,
        },
        checkbox, column, container, pick_list, text, text_input, Checkbox, Column, Container,
        Space, Text,
    },
    Color, Element, Length, Pixels, Point, Rectangle, Size, Subscription, Theme,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::env;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as ME};
use ui::{chart::calculate_knn_signals, CandleType, Candlestick, Chart, ChartState};

/*===============STRUCT LINE================= */
pub struct RTarde {
    candlesticks: BTreeMap<u64, Candlestick>,
    selected_coin: String,
    pub selected_candle_type: CandleType,
    coin_list: HashMap<String, CoinInfo>,
    auto_scroll: bool,
    ws_sender: Option<mpsc::Sender<String>>,
    show_ma5: bool,
    show_ma10: bool,
    show_ma20: bool,
    show_ma200: bool,
    loading_more: bool,          // 추가: 데이터 로딩 중인지 여부
    oldest_date: Option<String>, // 추가: 가장 오래된 캔들의 날짜
    knn_enabled: bool,
    knn_prediction: Option<String>,       // "UP" 또는 "DOWN"
    knn_buy_signals: BTreeMap<u64, f32>,  // bool에서 f32로 변경
    knn_sell_signals: BTreeMap<u64, f32>, // bool에서 f32로 변경
    account_info: Option<FuturesAccountInfo>,
    positions: Vec<Position>,
    alerts: VecDeque<Alert>,
    alert_timeout: Option<Instant>,
    auto_trading_enabled: bool,       // 자동매매 활성화 상태
    last_trade_time: Option<Instant>, // 마지막 거래 시간 (과도한 거래 방지용)
    alert_sender: mpsc::Sender<(String, AlertType)>,
    alert_receiver: mpsc::Receiver<(String, AlertType)>,
    average_prices: HashMap<String, f64>,
}
#[derive(Debug, Clone)]
struct Alert {
    message: String,
    alert_type: AlertType,
    timestamp: Instant,
}
#[derive(Debug, Clone)]
enum AlertType {
    Buy,   // 매수 신호
    Sell,  // 매도 신호
    Info,  // 일반 정보
    Error, // 에러
}
#[derive(Debug, Deserialize, Clone)]
struct Trade {
    symbol: String,
    id: u64,
    price: String,
    qty: String,
    #[serde(rename = "quoteQty")]
    quote_qty: String,
    #[serde(rename = "isBuyer")]
    is_buyer: bool,
    time: u64,
}
#[derive(Debug, Clone)]
pub enum Message {
    AddCandlestick((u64, BinanceTrade)),
    RemoveCandlestick,
    SelectCoin(String),
    UpdateCoinPrice(String, f64, f64),
    SelectCandleType(CandleType),
    Error,
    WebSocketInit(mpsc::Sender<String>),
    UpdatePrice(String, f64, f64),
    ToggleMA5,
    ToggleMA10,
    ToggleMA20,
    ToggleMA200,
    LoadMoreCandles,                               // 추가
    MoreCandlesLoaded(BTreeMap<u64, Candlestick>), // 추가
    ToggleKNN,                                     // KNN 시스템 켜기/끄기
    UpdateKNNPrediction(Option<String>),           // 예측 결과 업데이트
    TryBuy {
        price: f64,
        strength: f32,
        timestamp: u64,
        indicators: TradeIndicators, // 추가 지표 정보를 위한 구조체
    },
    TrySell {
        price: f64,
        strength: f32,
        timestamp: u64,
        indicators: TradeIndicators,
    },
    UpdateAccountInfo(FuturesAccountInfo),
    UpdatePositions(Vec<Position>),
    FetchError(String),
    AddAlert(String, AlertType),
    RemoveAlert,
    Tick,              // 알림 타이머용
    ToggleAutoTrading, // 자동매매 토글
    MarketBuy,         // 시장가 매수 메시지 추가
    MarketSell,        // 시장가 매도 메시지 추가
    UpdateAveragePrice(String, f64),
}
#[derive(Debug, Clone, Copy)]
enum TradeType {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub enum ScrollDirection {
    Left,
    Right,
}
#[derive(Debug, Clone)]
struct CoinInfo {
    symbol: String,
    name: String,
    price: f64,
}
// 계정 정보를 위한 구조체들

#[derive(Debug, Deserialize, Clone)]
pub struct FuturesPosition {
    symbol: String,
    #[serde(rename = "initialMargin")]
    initial_margin: String,
    #[serde(rename = "maintMargin")]
    maint_margin: String,
    #[serde(rename = "unrealizedProfit")]
    unrealized_profit: String,
    #[serde(rename = "positionInitialMargin")]
    position_initial_margin: String,
    leverage: String,
    isolated: bool,
    #[serde(rename = "entryPrice")]
    entry_price: String,
    #[serde(rename = "maxNotional")]
    max_notional: String,
    #[serde(rename = "positionSide")]
    position_side: String,
    #[serde(rename = "positionAmt")]
    position_amt: String,
}
#[derive(Debug, Deserialize, Clone)]
struct CommissionRates {
    maker: String,
    taker: String,
    buyer: String,
    seller: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Balance {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Position {
    symbol: String,
    #[serde(rename = "positionAmt")]
    position_amt: String,
    #[serde(rename = "entryPrice")]
    entry_price: String,
    #[serde(rename = "unrealizedProfit")]
    unrealized_profit: String,
    #[serde(rename = "liquidationPrice")]
    liquidation_price: String,
    leverage: String,
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

// 거래 지표 정보를 담는 구조체
#[derive(Debug, Clone)]
struct TradeIndicators {
    rsi: f32,
    ma5: f32,
    ma20: f32,
    volume_ratio: f32,
}
// KNN 예측기 최적화 버전
#[derive(Debug, Clone)]
struct OptimizedKNNPredictor {
    k: usize,
    window_size: usize,
    features_buffer: VecDeque<Vec<f32>>,
    labels_buffer: VecDeque<bool>,
    buffer_size: usize,
}

/*===============CONNECTION LINE================= */

async fn get_top_volume_pairs() -> Result<Vec<(String, f64)>, Box<dyn std::error::Error>> {
    let url = "https://fapi.binance.com/fapi/v1/ticker/24hr";

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let data: Vec<serde_json::Value> = response.json().await?;

    let mut pairs: Vec<(String, f64)> = data
        .into_iter()
        .filter(|item| {
            item["symbol"]
                .as_str()
                .map(|s| s.ends_with("USDT"))
                .unwrap_or(false)
        })
        .filter_map(|item| {
            let symbol = item["symbol"].as_str()?.to_string();
            let volume = item["quoteVolume"].as_str()?.parse::<f64>().ok()?;
            Some((symbol, volume))
        })
        .collect();

    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(pairs.into_iter().take(20).collect())
}

async fn get_symbol_info(symbol: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let url = "https://fapi.binance.com/fapi/v1/exchangeInfo";
    let response = reqwest::get(url).await?;
    let info: serde_json::Value = response.json().await?;

    if let Some(symbols) = info["symbols"].as_array() {
        for symbol_info in symbols {
            if symbol_info["symbol"].as_str() == Some(symbol) {
                let quantity_precision =
                    symbol_info["quantityPrecision"].as_u64().unwrap_or(3) as u32;
                let price_precision = symbol_info["pricePrecision"].as_u64().unwrap_or(2) as u32;
                return Ok((quantity_precision, price_precision));
            }
        }
    }

    Err("Symbol not found".into())
}
fn adjust_precision(value: f64, precision: u32) -> f64 {
    let scale = 10f64.powi(precision as i32);
    (value * scale).floor() / scale
}

/*===============RTarde LINE================= */
impl Default for RTarde {
    fn default() -> Self {
        // 거래량 상위 20개 코인 가져오기
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let top_pairs = runtime.block_on(async {
            match get_top_volume_pairs().await {
                Ok(pairs) => pairs,
                Err(e) => {
                    println!("Error fetching top pairs: {}", e);
                    vec![] // 에러 시 빈 벡터 반환
                }
            }
        });

        let mut coin_list = HashMap::new();

        // 상위 20개 코인으로 초기화
        for (symbol, _volume) in top_pairs {
            let symbol = symbol.strip_suffix("USDT").unwrap_or(&symbol);
            coin_list.insert(
                symbol.to_string(),
                CoinInfo {
                    symbol: format!("{}-USDT", symbol),
                    name: symbol.to_string(),
                    price: 0.0,
                },
            );
        }

        // 만약 API 호출이 실패하면 기본 리스트 사용
        if coin_list.is_empty() {
            for symbol in &[
                "BTC", "ETH", "XRP", "SOL", "DOT", "TRX", "TON", "SHIB", "DOGE", "PEPE", "BNB",
                "SUI", "XLM", "ADA",
            ] {
                coin_list.insert(
                    symbol.to_string(),
                    CoinInfo {
                        symbol: format!("{}-USDT", symbol),
                        name: symbol.to_string(),
                        price: 0.0,
                    },
                );
            }
        }

        let (alert_sender, alert_receiver) = mpsc::channel(100);

        Self {
            candlesticks: fetch_candles("USDT-BTC", &CandleType::Day, None).unwrap_or_default(),
            selected_coin: "BTC".to_string(),
            selected_candle_type: CandleType::Day,
            coin_list,
            auto_scroll: true,
            ws_sender: None,
            show_ma5: false,
            show_ma10: false,
            show_ma20: false,
            show_ma200: false,
            loading_more: false,
            oldest_date: None,
            knn_enabled: false,
            knn_prediction: None,
            knn_buy_signals: BTreeMap::new(),
            knn_sell_signals: BTreeMap::new(),
            account_info: None,
            positions: Vec::new(),
            alerts: VecDeque::with_capacity(5),
            alert_timeout: None,
            auto_trading_enabled: false,
            last_trade_time: None,
            alert_sender,
            alert_receiver,
            average_prices: HashMap::new(),
        }
    }
}
impl RTarde {
    fn binance_account_subscription(&self) -> Subscription<Message> {
        Subscription::run(binance_account_connection)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // 기존 웹소켓 subscription
            self.websocket_subscription(),
            self.binance_account_subscription(),
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick),
        ])
    }
    fn websocket_subscription(&self) -> Subscription<Message> {
        Subscription::run(binance_connection)
    }

    pub fn view(&self) -> Element<Message> {
        let ma_controls = Container::new(
            Column::new()
                .spacing(5)
                .push(
                    Row::new()
                        .spacing(10)
                        .push(checkbox("MA5", self.show_ma5).on_toggle(|_| Message::ToggleMA5))
                        .push(checkbox("MA10", self.show_ma10).on_toggle(|_| Message::ToggleMA10)),
                )
                .push(
                    Row::new()
                        .spacing(10)
                        .push(checkbox("MA20", self.show_ma20).on_toggle(|_| Message::ToggleMA20))
                        .push(
                            checkbox("MA200", self.show_ma200).on_toggle(|_| Message::ToggleMA200),
                        ),
                )
                .push(Row::new().spacing(10).push(
                    checkbox("KNN prediction", self.knn_enabled).on_toggle(|_| Message::ToggleKNN),
                )),
        )
        .padding(10);

        let prediction_display = Container::new(Column::new().push(
            if let Some(alert) = self.alerts.front() {
                Text::new(&alert.message).color(match alert.alert_type {
                    AlertType::Buy => Color::from_rgb(0.0, 0.8, 0.0),
                    AlertType::Sell => Color::from_rgb(0.8, 0.0, 0.0),
                    AlertType::Info => Color::from_rgb(0.0, 0.0, 0.8),
                    AlertType::Error => Color::from_rgb(0.8, 0.0, 0.0),
                })
            } else {
                Text::new("")
            },
        ))
        .padding(10)
        .width(Length::Shrink)
        .height(Length::Shrink);

        let coins: Vec<String> = self.coin_list.keys().cloned().collect();
        let coin_picker = pick_list(coins, Some(self.selected_coin.clone()), Message::SelectCoin)
            .width(Length::Fixed(150.0));

        let candle_types = vec![CandleType::Minute1, CandleType::Minute3, CandleType::Day];
        let candle_type_strings: Vec<String> =
            candle_types.iter().map(|ct| ct.to_string()).collect();
        let candle_type_picker = pick_list(
            candle_type_strings,
            Some(self.selected_candle_type.to_string()),
            |s| {
                let candle_type = match s.as_str() {
                    "1Minute" => CandleType::Minute1,
                    "3Minute" => CandleType::Minute3,
                    "Day" => CandleType::Day,
                    _ => CandleType::Day,
                };
                Message::SelectCandleType(candle_type)
            },
        )
        .width(Length::Fixed(100.0));

        let current_coin_info = if let Some(info) = self.coin_list.get(&self.selected_coin) {
            let avg_price = self
                .average_prices
                .get(&self.selected_coin)
                .copied()
                .unwrap_or(0.0);
            let profit_percentage = if avg_price > 0.0 {
                ((info.price - avg_price) / avg_price * 100.0)
            } else {
                0.0
            };

            let profit_color = if profit_percentage >= 0.0 {
                Color::from_rgb(0.0, 0.8, 0.0)
            } else {
                Color::from_rgb(0.8, 0.0, 0.0)
            };

            let position_info = self.account_info.as_ref().and_then(|account| {
                account
                    .positions
                    .iter()
                    .find(|pos| pos.symbol == format!("{}USDT", self.selected_coin))
            });

            Column::new()
                .spacing(10)
                .push(
                    Container::new(
                        Column::new()
                            .push(Text::new(&info.name).size(28).width(Length::Fill))
                            .push(
                                Text::new(&info.symbol)
                                    .size(14)
                                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
                            ),
                    )
                    .padding(10)
                    .width(Length::Fill),
                )
                .push(
                    Container::new(Text::new(format!("{:.6} USDT", info.price)).size(32))
                        .padding(15)
                        .width(Length::Fill),
                )
                .push(
                    Container::new(
                        Column::new()
                            .push(
                                Text::new(format!(
                                    "valuation profit and loss: {:.2}%",
                                    profit_percentage
                                ))
                                .size(20)
                                .color(profit_color),
                            )
                            .push(
                                Row::new()
                                    .spacing(10)
                                    .push(Text::new("account balance:"))
                                    .push(
                                        Text::new(if let Some(account) = &self.account_info {
                                            if let Some(asset) =
                                                account.assets.iter().find(|a| a.asset == "USDT")
                                            {
                                                let available = asset
                                                    .available_balance
                                                    .parse::<f64>()
                                                    .unwrap_or(0.0);
                                                let unrealized = asset
                                                    .unrealized_profit
                                                    .parse::<f64>()
                                                    .unwrap_or(0.0);
                                                format!(
                                                    "{:.2} USDT (unrealized: {:.2})",
                                                    available, unrealized
                                                )
                                            } else {
                                                "0.00 USDT".to_string()
                                            }
                                        } else {
                                            "Loading...".to_string()
                                        })
                                        .size(16),
                                    ),
                            )
                            .push(
                                Row::new().spacing(10).push(Text::new("position:")).push(
                                    Text::new(if let Some(pos) = position_info {
                                        let amt = pos.position_amt.parse::<f64>().unwrap_or(0.0);
                                        let entry = pos.entry_price.parse::<f64>().unwrap_or(0.0);
                                        let pnl =
                                            pos.unrealized_profit.parse::<f64>().unwrap_or(0.0);
                                        if amt != 0.0 {
                                            format!(
                                                "{:.3} (approach: {:.2}, PNL: {:.2})",
                                                amt, entry, pnl
                                            )
                                        } else {
                                            "doesn't exist".to_string()
                                        }
                                    } else {
                                        "doesn't exist".to_string()
                                    })
                                    .size(16),
                                ),
                            ),
                    )
                    .padding(10)
                    .width(Length::Fill),
                )
        } else {
            Column::new().push(Text::new("Loading..."))
        };
        let position_label = format!("{} Position:", self.selected_coin);
        let position_text = if let Some(info) = &self.account_info {
            let symbol = format!("{}USDT", self.selected_coin);
            if let Some(position) = info.positions.iter().find(|p| p.symbol == symbol) {
                let amt = position.position_amt.parse::<f64>().unwrap_or(0.0);
                let entry = position.entry_price.parse::<f64>().unwrap_or(0.0);
                let pnl = position.unrealized_profit.parse::<f64>().unwrap_or(0.0);
                if amt != 0.0 {
                    let direction = if amt > 0.0 { "Long" } else { "Short" };
                    format!(
                        "{} {:.8} @ {:.2} (PNL: {:.2})",
                        direction,
                        amt.abs(),
                        entry,
                        pnl
                    )
                } else {
                    "No Position".to_string()
                }
            } else {
                "No Position".to_string()
            }
        } else {
            "Loading...".to_string()
        };
        let canvas = Canvas::new(Chart::new(
            self.candlesticks.clone(),
            self.selected_candle_type.clone(),
            self.show_ma5,
            self.show_ma10,
            self.show_ma20,
            self.show_ma200,
            self.knn_enabled,
            self.knn_prediction.clone(),
            self.knn_buy_signals.clone(),
            self.knn_sell_signals.clone(),
        ))
        .width(iced::Fill)
        .height(Length::from(800));

        let auto_trading_toggle = Container::new(
            Row::new()
                .spacing(10)
                .push(
                    checkbox("Auto trading", self.auto_trading_enabled)
                        .on_toggle(|_| Message::ToggleAutoTrading),
                )
                .push(
                    Text::new(if self.auto_trading_enabled {
                        "Auto trading on"
                    } else {
                        "Auto trading off"
                    })
                    .size(14)
                    .color(if self.auto_trading_enabled {
                        Color::from_rgb(0.0, 0.8, 0.0)
                    } else {
                        Color::from_rgb(0.5, 0.5, 0.5)
                    }),
                ),
        )
        .padding(10)
        .width(Length::Fill);
        let right_side_bar = Column::new()
            .spacing(20)
            .padding(20)
            .push(auto_trading_toggle)
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("Account Info").size(24))
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(Text::new("Total Balance:"))
                            .push(
                                Text::new(if let Some(info) = &self.account_info {
                                    if let Some(asset) =
                                        info.assets.iter().find(|a| a.asset == "USDT")
                                    {
                                        let balance =
                                            asset.wallet_balance.parse::<f64>().unwrap_or(0.0);
                                        let pnl =
                                            asset.unrealized_profit.parse::<f64>().unwrap_or(0.0);
                                        format!("{:.2} USDT (PNL: {:.2})", balance, pnl)
                                    } else {
                                        "0.00 USDT".to_string()
                                    }
                                } else {
                                    "Loading...".to_string()
                                })
                                .size(16),
                            ),
                    ),
            )
            .push(
                Container::new(Text::new("Order").size(24))
                    .width(Length::Fill)
                    .center_x(0),
            )
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("Order Type").size(16))
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(
                                button(Text::new("Long Market"))
                                    .width(Length::Fill)
                                    .on_press(Message::MarketBuy),
                            )
                            .push(
                                button(Text::new("Short Market"))
                                    .width(Length::Fill)
                                    .on_press(Message::MarketSell),
                            ),
                    ),
            )
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("Limit Order").size(16))
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(text_input("Enter price...", ""))
                            .push(Text::new("USDT")),
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(text_input("Enter quantity...", ""))
                            .push(Text::new(self.selected_coin.clone())),
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(button(Text::new("Long Limit")).width(Length::Fill))
                            .push(button(Text::new("Short Limit")).width(Length::Fill)),
                    ),
            )
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("Position Size").size(16))
                    .push(
                        Row::new()
                            .spacing(5)
                            .push(button(Text::new("10%")).width(Length::Fill))
                            .push(button(Text::new("25%")).width(Length::Fill))
                            .push(button(Text::new("50%")).width(Length::Fill))
                            .push(button(Text::new("100%")).width(Length::Fill)),
                    ),
            )
            .push(Space::with_height(Length::Fill))
            .push(Container::new(
                Column::new()
                    .spacing(10)
                    .push(Text::new("Current Positions").size(16))
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(Text::new("USDT Balance:"))
                            .push(
                                Text::new(if let Some(info) = &self.account_info {
                                    if let Some(asset) =
                                        info.assets.iter().find(|a| a.asset == "USDT")
                                    {
                                        let available =
                                            asset.available_balance.parse::<f64>().unwrap_or(0.0);
                                        format!("{:.2}", available)
                                    } else {
                                        "0.00".to_string()
                                    }
                                } else {
                                    "Loading...".to_string()
                                })
                                .size(16),
                            ),
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(Text::new(format!("{} Position:", self.selected_coin)).size(16)) // 직접 format
                            .push(
                                Text::new(if let Some(info) = &self.account_info {
                                    let symbol = format!("{}USDT", self.selected_coin);
                                    if let Some(position) =
                                        info.positions.iter().find(|p| p.symbol == symbol)
                                    {
                                        let amt =
                                            position.position_amt.parse::<f64>().unwrap_or(0.0);
                                        let entry =
                                            position.entry_price.parse::<f64>().unwrap_or(0.0);
                                        let pnl = position
                                            .unrealized_profit
                                            .parse::<f64>()
                                            .unwrap_or(0.0);
                                        if amt != 0.0 {
                                            let direction =
                                                if amt > 0.0 { "Long" } else { "Short" };
                                            format!(
                                                "{} {:.8} @ {:.2} (PNL: {:.2})",
                                                direction,
                                                amt.abs(),
                                                entry,
                                                pnl
                                            )
                                        } else {
                                            "No Position".to_string()
                                        }
                                    } else {
                                        "No Position".to_string()
                                    }
                                } else {
                                    "Loading...".to_string()
                                })
                                .size(16),
                            ),
                    ),
            ));

        let left_side_bar = Column::new()
            .spacing(20)
            .padding(20)
            .push(current_coin_info);

        Column::new()
            .push(
                Row::new()
                    .push(coin_picker.width(FillPortion(1)))
                    .push(candle_type_picker.width(FillPortion(1)))
                    .push(ma_controls.width(FillPortion(8)))
                    .push(prediction_display.width(FillPortion(2))),
            )
            .push(
                Row::new()
                    .push(container(left_side_bar).width(FillPortion(1)))
                    .push(container(canvas).width(FillPortion(4)))
                    .push(container(right_side_bar).width(FillPortion(1))),
            )
            .into()
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::UpdateAveragePrice(symbol, price) => {
                self.average_prices.insert(symbol, price);
            }
            Message::MarketBuy => {
                if let Some(info) = self.coin_list.get(&self.selected_coin) {
                    if let Some(account_info) = &self.account_info {
                        let price = info.price;
                        let fixed_usdt = 5.5; // 5.5 USDT로 고정

                        // 매수 수량 계산 (USDT / 현재가)
                        let quantity = fixed_usdt / price;

                        // 수량이 0보다 큰지 확인
                        if quantity > 0.0 {
                            println!("Calculated quantity: {}", quantity); // 디버그용

                            let selected_coin = self.selected_coin.clone();
                            let alert_sender = self.alert_sender.clone();

                            let runtime = tokio::runtime::Handle::current();
                            runtime.spawn(async move {
                                if let Err(e) = execute_trade(
                                    selected_coin.clone(),
                                    TradeType::Buy,
                                    price,
                                    quantity,
                                    alert_sender,
                                )
                                .await
                                {
                                    println!("시장가 매수 실패: {:?}", e);
                                }
                            });

                            self.add_alert(
                                format!(
                                    "시장가 매수(롱) 시도:\n수량: {:.8} {}\n예상 비용: {:.4} USDT",
                                    quantity, self.selected_coin, fixed_usdt
                                ),
                                AlertType::Info,
                            );
                        } else {
                            self.add_alert(
                                format!(
                                    "주문 실패: 계산된 수량이 너무 작습니다 (가격: {} USDT)",
                                    price
                                ),
                                AlertType::Error,
                            );
                        }
                    } else {
                        self.add_alert(
                            "계정 정보를 불러올 수 없습니다.".to_string(),
                            AlertType::Error,
                        );
                    }
                }
            }

            Message::MarketSell => {
                if let Some(info) = self.coin_list.get(&self.selected_coin) {
                    if let Some(account_info) = &self.account_info {
                        let price = info.price;
                        let fixed_usdt = 5.5; // 5.5 USDT로 고정

                        // 매도 수량 계산 (USDT / 현재가)
                        let quantity = fixed_usdt / price;

                        // 수량이 0보다 큰지 확인
                        if quantity > 0.0 {
                            println!("Calculated quantity: {}", quantity); // 디버그용

                            let selected_coin = self.selected_coin.clone();
                            let alert_sender = self.alert_sender.clone();

                            let runtime = tokio::runtime::Handle::current();
                            runtime.spawn(async move {
                                if let Err(e) = execute_trade(
                                    selected_coin.clone(),
                                    TradeType::Sell,
                                    price,
                                    quantity,
                                    alert_sender,
                                )
                                .await
                                {
                                    println!("시장가 매도 실패: {:?}", e);
                                }
                            });

                            self.add_alert(
                                format!(
                                    "시장가 매도(숏) 시도:\n수량: {:.8} {}\n예상 비용: {:.4} USDT",
                                    quantity, self.selected_coin, fixed_usdt
                                ),
                                AlertType::Info,
                            );
                        } else {
                            self.add_alert(
                                format!(
                                    "주문 실패: 계산된 수량이 너무 작습니다 (가격: {} USDT)",
                                    price
                                ),
                                AlertType::Error,
                            );
                        }
                    } else {
                        self.add_alert(
                            "계정 정보를 불러올 수 없습니다.".to_string(),
                            AlertType::Error,
                        );
                    }
                }
            }
            Message::ToggleAutoTrading => {
                self.auto_trading_enabled = !self.auto_trading_enabled;
                let status = if self.auto_trading_enabled {
                    "Automatic trading activate"
                } else {
                    "Automatic trading deactivate"
                };
                self.add_alert(format!("{}", status), AlertType::Info);
            }

            Message::TryBuy {
                price,
                strength,
                timestamp,
                indicators,
            } => {
                let dt = chrono::DateTime::from_timestamp((timestamp / 1000) as i64, 0)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Local);

                println!("=== 강한 매수 신호 감지! ===");
                println!("시간: {}", dt.format("%Y-%m-%d %H:%M:%S"));
                println!("코인: {}", self.selected_coin);
                println!("가격: {:.2} USDT", price);
                println!("신호 강도: {:.2}", strength);
                println!("RSI: {:.2}", indicators.rsi);
                println!("MA5/MA20: {:.2}/{:.2}", indicators.ma5, indicators.ma20);
                println!("거래량 비율: {:.2}", indicators.volume_ratio);
                println!("========================");

                self.add_alert(
                    format!(
                        "매수 신호 감지!\n가격: {:.2} USDT\n강도: {:.2}\nRSI: {:.2}",
                        price, strength, indicators.rsi
                    ),
                    AlertType::Buy,
                );

                if self.auto_trading_enabled {
                    let can_trade = self
                        .last_trade_time
                        .map(|time| time.elapsed() > Duration::from_secs(60))
                        .unwrap_or(true);

                    if can_trade {
                        let amount = 0.001;
                        let selected_coin = self.selected_coin.clone();
                        let alert_sender = self.alert_sender.clone();

                        let runtime = tokio::runtime::Handle::current();
                        runtime.spawn(async move {
                            if let Err(e) = execute_trade(
                                selected_coin,
                                TradeType::Buy,
                                price,
                                amount,
                                alert_sender,
                            )
                            .await
                            {
                                println!("매수 실패: {:?}", e);
                            }
                        });

                        self.last_trade_time = Some(Instant::now());
                    }
                }
            }

            Message::TrySell {
                price,
                strength,
                timestamp,
                indicators,
            } => {
                let dt = chrono::DateTime::from_timestamp((timestamp / 1000) as i64, 0)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Local);

                println!("=== 강한 매도 신호 감지! ===");
                println!("시간: {}", dt.format("%Y-%m-%d %H:%M:%S"));
                println!("코인: {}", self.selected_coin);
                println!("가격: {:.2} USDT", price);
                println!("신호 강도: {:.2}", strength);
                println!("RSI: {:.2}", indicators.rsi);
                println!("MA5/MA20: {:.2}/{:.2}", indicators.ma5, indicators.ma20);
                println!("거래량 비율: {:.2}", indicators.volume_ratio);
                println!("========================");

                self.add_alert(
                    format!(
                        "매도 신호 감지!\n가격: {:.2} USDT\n강도: {:.2}\nRSI: {:.2}",
                        price, strength, indicators.rsi
                    ),
                    AlertType::Sell,
                );

                if self.auto_trading_enabled {
                    let can_trade = self
                        .last_trade_time
                        .map(|time| time.elapsed() > Duration::from_secs(60))
                        .unwrap_or(true);

                    if can_trade {
                        let amount = 0.001;
                        let selected_coin = self.selected_coin.clone();
                        let alert_sender = self.alert_sender.clone();

                        let runtime = tokio::runtime::Handle::current();
                        runtime.spawn(async move {
                            if let Err(e) = execute_trade(
                                selected_coin,
                                TradeType::Sell,
                                price,
                                amount,
                                alert_sender,
                            )
                            .await
                            {
                                println!("매도 실패: {:?}", e);
                            }
                        });

                        self.last_trade_time = Some(Instant::now());
                    }
                }
            }
            Message::UpdateAccountInfo(info) => {
                self.account_info = Some(info);
            }
            Message::UpdatePositions(positions) => {
                self.positions = positions;
            }
            Message::FetchError(error) => {
                println!("API Error: {}", error);
            }

            Message::AddAlert(message, alert_type) => {
                self.alerts.push_back(Alert {
                    message,
                    alert_type,
                    timestamp: Instant::now(),
                });
            }

            Message::RemoveAlert => {
                self.alerts.pop_front();
            }

            Message::Tick => {
                // 5초 이상 된 알림 제거
                while let Some(alert) = self.alerts.front() {
                    if alert.timestamp.elapsed() > Duration::from_secs(5) {
                        self.alerts.pop_front();
                    } else {
                        break;
                    }
                }
            }

            Message::ToggleKNN => {
                self.knn_enabled = !self.knn_enabled;
                if self.knn_enabled {
                    if let Some(prediction) = self.predict_knn() {
                        self.knn_prediction = Some(prediction);
                        let (buy_signals, sell_signals) =
                            calculate_knn_signals(&self.candlesticks, false);
                        self.knn_buy_signals = buy_signals;
                        self.knn_sell_signals = sell_signals;
                    }
                } else {
                    self.knn_prediction = None;
                    self.knn_buy_signals.clear();
                    self.knn_sell_signals.clear();
                }
            }

            Message::UpdateKNNPrediction(prediction) => {
                self.knn_prediction = prediction;
            }

            Message::LoadMoreCandles => {
                if !self.loading_more {
                    // 가장 오래된 캔들의 날짜를 찾아서 to 파라미터로 사용
                    if let Some((&oldest_timestamp, _)) = self.candlesticks.iter().next() {
                        self.loading_more = true;
                        let datetime = chrono::NaiveDateTime::from_timestamp_opt(
                            (oldest_timestamp / 1000) as i64,
                            0,
                        )
                        .unwrap();
                        let date_str = datetime.format("%Y-%m-%dT%H:%M:%S").to_string();

                        // 클론해서 async 클로저에 전달
                        let market = format!("USDT-{}", self.selected_coin);
                        let candle_type = self.selected_candle_type.clone();

                        let runtime = tokio::runtime::Handle::current();
                        runtime.spawn(async move {
                            match fetch_candles_async(&market, &candle_type, Some(date_str)).await {
                                Ok(new_candles) => Message::MoreCandlesLoaded(new_candles),
                                Err(_) => Message::Error,
                            }
                        });
                    }
                }
            }
            Message::MoreCandlesLoaded(mut new_candles) => {
                if !new_candles.is_empty() {
                    self.candlesticks.append(&mut new_candles);

                    // 새로운 데이터가 로드되면 KNN 신호도 다시 계산
                    if self.knn_enabled {
                        let (buy_signals, sell_signals) =
                            calculate_knn_signals(&self.candlesticks, false); // false 추가
                        self.knn_buy_signals = buy_signals;
                        self.knn_sell_signals = sell_signals;
                    }
                }
            }
            Message::ToggleMA5 => self.show_ma5 = !self.show_ma5,
            Message::ToggleMA10 => self.show_ma10 = !self.show_ma10,
            Message::ToggleMA20 => self.show_ma20 = !self.show_ma20,
            Message::ToggleMA200 => self.show_ma200 = !self.show_ma200,
            Message::SelectCandleType(candle_type) => {
                self.add_alert(format!("1"), AlertType::Buy);
                println!("Changing candle type to: {}", candle_type);
                self.selected_candle_type = candle_type.clone();

                // 캔들스틱 데이터 새로 불러오기
                let market = format!("USDT-{}", self.selected_coin);
                println!(
                    "Fetching new candles for market {} with type {}",
                    market, candle_type
                );

                match fetch_candles(&market, &candle_type, None) {
                    // None을 추가하여 최신 데이터부터 가져오기
                    Ok(candles) => {
                        println!(
                            "Successfully fetched {} candles for {}",
                            candles.len(),
                            candle_type
                        );
                        self.candlesticks = candles;
                        // KNN 활성화 상태면 과거 데이터에 대해서도 신호 계산
                        if self.knn_enabled {
                            let (buy_signals, sell_signals) =
                                calculate_knn_signals(&self.candlesticks, false);
                            self.knn_buy_signals = buy_signals;
                            self.knn_sell_signals = sell_signals;

                            // 예측도 업데이트
                            if let Some(prediction) = self.predict_knn() {
                                self.knn_prediction = Some(prediction);
                            }
                        }
                        // 가장 오래된 캔들의 날짜 저장
                        if let Some((&timestamp, _)) = self.candlesticks.iter().next() {
                            let datetime = chrono::NaiveDateTime::from_timestamp_opt(
                                (timestamp / 1000) as i64,
                                0,
                            )
                            .unwrap();
                            self.oldest_date =
                                Some(datetime.format("%Y-%m-%dT%H:%M:%S").to_string());
                        } else {
                            self.oldest_date = None;
                        }

                        self.auto_scroll = true;
                    }
                    Err(e) => {
                        println!("Error fetching {} candles: {:?}", candle_type, e);
                    }
                }
            }
            Message::UpdatePrice(symbol, price, change_rate) => {
                if let Some(info) = self.coin_list.get_mut(&symbol) {
                    // info.prev_price = info.price;
                    info.price = price;
                    // info.change_percent = change_rate;
                    // println!("Price updated for {}: {} ({}%)", symbol, price, change_rate);
                }
            }
            Message::WebSocketInit(sender) => {
                self.ws_sender = Some(sender);
            }
            Message::SelectCoin(symbol) => {
                println!("Switching to coin: {}", symbol);
                self.selected_coin = symbol.clone();
                self.candlesticks.clear();

                match fetch_candles(
                    &format!("USDT-{}", symbol),
                    &self.selected_candle_type,
                    None,
                ) {
                    Ok(candles) => {
                        if candles.is_empty() {
                            println!("Warning: No candles received for {}", symbol);
                        } else {
                            println!(
                                "Successfully loaded {} candles for {}",
                                candles.len(),
                                symbol
                            );
                            self.candlesticks = candles;
                            // KNN 활성화 상태면 과거 데이터에 대해서도 신호 계산
                            if self.knn_enabled {
                                let (buy_signals, sell_signals) =
                                    calculate_knn_signals(&self.candlesticks, false);
                                self.knn_buy_signals = buy_signals;
                                self.knn_sell_signals = sell_signals;

                                // 예측도 업데이트
                                if let Some(prediction) = self.predict_knn() {
                                    self.knn_prediction = Some(prediction);
                                }
                            }
                            // 가장 오래된 캔들의 날짜 저장
                            if let Some((&timestamp, _)) = self.candlesticks.iter().next() {
                                let datetime = chrono::NaiveDateTime::from_timestamp_opt(
                                    (timestamp / 1000) as i64,
                                    0,
                                )
                                .unwrap();
                                self.oldest_date =
                                    Some(datetime.format("%Y-%m-%dT%H:%M:%S").to_string());
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error fetching candles for {}: {:?}", symbol, e);
                    }
                }

                if let Some(sender) = &self.ws_sender {
                    if let Err(e) = sender.clone().try_send(symbol.clone()) {
                        println!("Error sending WebSocket subscription: {:?}", e);
                    }
                }
                self.auto_scroll = true;
            }
            Message::UpdateCoinPrice(symbol, price, change) => {
                if let Some(info) = self.coin_list.get_mut(&symbol) {
                    info.price = price;
                }
            }
            Message::AddCandlestick(trade) => {
                let (timestamp, trade_data) = trade;
                let current_market = format!("{}USDT", self.selected_coin);

                if trade_data.symbol != current_market {
                    return;
                }

                if self.knn_enabled {
                    let (buy_signals, sell_signals) =
                        calculate_knn_signals(&self.candlesticks, true); // true로 실시간 표시
                    self.knn_buy_signals = buy_signals;
                    self.knn_sell_signals = sell_signals;
                }
                if self.candlesticks.is_empty() {
                    // 초기 데이터 로드
                    if let Ok(candles) = fetch_candles(
                        &format!("USDT-{}", self.selected_coin),
                        &self.selected_candle_type,
                        None,
                    ) {
                        self.candlesticks = candles;
                    }
                }

                let current_timestamp = timestamp;
                let candle_timestamp = match self.selected_candle_type {
                    CandleType::Minute1 => current_timestamp - (current_timestamp % 60000),
                    CandleType::Minute3 => current_timestamp - (current_timestamp % 180000),
                    CandleType::Day => current_timestamp - (current_timestamp % 86400000),
                };

                let trade_price = trade_data.price.parse::<f32>().unwrap_or_default();
                let trade_volume = trade_data.quantity.parse::<f32>().unwrap_or_default();

                self.candlesticks
                    .entry(candle_timestamp)
                    .and_modify(|candle| {
                        candle.high = candle.high.max(trade_price);
                        candle.low = candle.low.min(trade_price);
                        candle.close = trade_price;
                        candle.volume += trade_volume;
                    })
                    .or_insert(Candlestick {
                        open: trade_price,
                        high: trade_price,
                        low: trade_price,
                        close: trade_price,
                        volume: trade_volume,
                    });
                self.auto_scroll = true;
            }
            Message::RemoveCandlestick => {
                if let Some(&last_key) = self.candlesticks.keys().last() {
                    self.candlesticks.remove(&last_key);
                }
                self.auto_scroll = true;
            }

            Message::Error => {
                println!("WebSocket connection error");
            }
        }
    }

    // KNN 예측 헬퍼 메서드
    fn predict_knn(&self) -> Option<String> {
        let predictor = OptimizedKNNPredictor::new(5, 20, 1000);
        let data: Vec<(&u64, &Candlestick)> = self.candlesticks.iter().collect();
        if data.len() < predictor.window_size {
            return None;
        }

        if let Some(features) =
            predictor.extract_features(&data[data.len() - predictor.window_size..])
        {
            predictor.predict(&features)
        } else {
            None
        }
    }

    fn add_alert(&mut self, message: String, alert_type: AlertType) {
        self.alerts.push_back(Alert {
            message,
            alert_type,
            timestamp: Instant::now(),
        });

        // 최대 5개까지만 유지
        while self.alerts.len() > 5 {
            self.alerts.pop_front();
        }
    }
}
/*===============CHART LINE================= */

impl std::fmt::Display for CandleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandleType::Minute1 => write!(f, "1Minute"),
            CandleType::Minute3 => write!(f, "3Minute"), // 표시 텍스트 변경
            CandleType::Day => write!(f, "Day"),
        }
    }
}
/*===============UTILS LINE================= */
trait VecDequeExt<T: 'static> {
    fn range(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = &(u64, T)>;
}

impl<T: 'static> VecDequeExt<T> for VecDeque<(u64, T)> {
    fn range(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = &(u64, T)> {
        self.iter().skip(range.start).take(range.end - range.start)
    }
}

impl OptimizedKNNPredictor {
    fn new(k: usize, window_size: usize, buffer_size: usize) -> Self {
        Self {
            k,
            window_size,
            features_buffer: VecDeque::with_capacity(buffer_size),
            labels_buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
        }
    }

    // 특성 추출 최적화
    fn extract_features(&self, candlesticks: &[(&u64, &Candlestick)]) -> Option<Vec<f32>> {
        if candlesticks.len() < self.window_size {
            return None;
        }

        let mut features = Vec::with_capacity(self.window_size * 4);

        // 가격 변화율 계산
        let mut price_changes = Vec::with_capacity(self.window_size - 1);
        for window in candlesticks.windows(2) {
            let price_change =
                ((window[1].1.close - window[0].1.close) / window[0].1.close) * 100.0;
            price_changes.push(price_change);
        }

        // 기술적 지표 계산
        let (ma5, ma20) = self.calculate_moving_averages(candlesticks);
        let rsi = self.calculate_rsi(&price_changes, 14);
        let volume_ratio = self.calculate_volume_ratio(candlesticks);

        // 특성 결합
        features.extend_from_slice(&[
            ma5 / ma20 - 1.0,                             // MA 비율
            rsi / 100.0,                                  // 정규화된 RSI
            volume_ratio,                                 // 거래량 비율
            price_changes.last().unwrap_or(&0.0) / 100.0, // 최근 가격 변화
        ]);

        Some(features)
    }

    // 이동평균 계산 최적화
    fn calculate_moving_averages(&self, data: &[(&u64, &Candlestick)]) -> (f32, f32) {
        let (ma5, ma20) = data
            .iter()
            .rev()
            .take(20)
            .fold((0.0, 0.0), |acc, (_, candle)| {
                (
                    if data.len() >= 5 {
                        acc.0 + candle.close / 5.0
                    } else {
                        acc.0
                    },
                    acc.1 + candle.close / 20.0,
                )
            });
        (ma5, ma20)
    }

    // RSI 계산 최적화
    fn calculate_rsi(&self, price_changes: &[f32], period: usize) -> f32 {
        let (gains, losses): (Vec<_>, Vec<_>) = price_changes
            .iter()
            .map(|&change| {
                if change > 0.0 {
                    (change, 0.0)
                } else {
                    (0.0, -change)
                }
            })
            .unzip();

        let avg_gain: f32 = gains.iter().sum::<f32>() / period as f32;
        let avg_loss: f32 = losses.iter().sum::<f32>() / period as f32;

        if avg_loss == 0.0 {
            100.0
        } else {
            100.0 - (100.0 / (1.0 + (avg_gain / avg_loss)))
        }
    }

    // 거래량 비율 계산
    fn calculate_volume_ratio(&self, data: &[(&u64, &Candlestick)]) -> f32 {
        let recent_volume = data.last().map(|(_, c)| c.volume).unwrap_or(0.0);
        let avg_volume = data.iter().map(|(_, c)| c.volume).sum::<f32>() / data.len() as f32;
        recent_volume / avg_volume
    }

    // 예측 최적화
    fn predict(&self, features: &[f32]) -> Option<String> {
        if self.features_buffer.is_empty() {
            return None;
        }

        let mut distances: Vec<(f32, bool)> = self
            .features_buffer
            .iter()
            .zip(self.labels_buffer.iter())
            .map(|(train_features, &label)| {
                let distance = self.euclidean_distance(features, train_features);
                (distance, label)
            })
            .collect();

        distances.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let up_votes = distances
            .iter()
            .take(self.k)
            .filter(|&&(_, label)| label)
            .count();

        Some(if up_votes > self.k / 2 { "▲" } else { "▼" }.to_string())
    }

    // 거리 계산 최적화
    fn euclidean_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}

fn main() -> iced::Result {
    dotenv().ok();

    iced::application("Candlestick Chart", RTarde::update, RTarde::view)
        .subscription(RTarde::subscription)
        .window_size(Size::new(1980., 1080.))
        .run()
}
