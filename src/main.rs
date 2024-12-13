use dotenv::dotenv;
use models::OptimizedKNNPredictor;
mod api;
mod models;
mod trading;
mod ui;
mod utils;
use api::{
    account::binance_account_connection,
    binance::{binance_connection, fetch_candles, fetch_candles_async, get_top_volume_pairs},
    excution::execute_trade,
    BinanceTrade, FuturesAccountInfo,
};
use iced::{
    futures::channel::mpsc,
    time::{Duration, Instant},
    widget::{canvas::Canvas, container, pick_list, Column, Container, Row, Text},
    Element, Length,
    Length::FillPortion,
    Size, Subscription,
};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, VecDeque};
use trading::{
    markey_order::{market_buy, market_sell},
    TradeType,
};
use ui::{
    buttons::ma_controls,
    chart::{calculate_knn_signals, calculate_momentum_signals},
    infos::{account_info, coin_info, current_position},
    trading::{auto_trading_toggle, order_buttons},
    CandleType, Candlestick, Chart, ChartState,
};
use utils::{constant as uc, logs as ul};

pub struct Futurx {
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
    alerts: VecDeque<Alert>,
    auto_trading_enabled: bool,       // 자동매매 활성화 상태
    last_trade_time: Option<Instant>, // 마지막 거래 시간 (과도한 거래 방지용)
    alert_sender: mpsc::Sender<(String, AlertType)>,
    average_prices: HashMap<String, f64>,
    momentum_enabled: bool,
    momentum_buy_signals: BTreeMap<u64, f32>, // bool에서 f32로 변경
    momentum_sell_signals: BTreeMap<u64, f32>, // bool에서 f32로 변경
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
    UpdatePrice(String, f64, f64),                 //가격업데이트
    ToggleMA5,                                     //5일 이동평균선
    ToggleMA10,                                    //10일 이동평균선
    ToggleMA20,                                    //20일 이동평균선
    ToggleMA200,                                   //200일 이동평균선
    LoadMoreCandles,                               // 추가
    MoreCandlesLoaded(BTreeMap<u64, Candlestick>), // 추가
    ToggleKNN,
    ToggleMomentum, // KNN 시스템 켜기/끄기
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
    FetchError(String),
    AddAlert(String, AlertType),
    RemoveAlert,
    Tick,              // 알림 타이머용
    ToggleAutoTrading, // 자동매매 토글
    MarketBuy,         // 시장가 매수 메시지 추가
    MarketSell,        // 시장가 매도 메시지 추가
    UpdateAveragePrice(String, f64),
}

#[derive(Debug, Clone)]
struct CoinInfo {
    symbol: String,
    name: String,
    price: f64,
}

// 거래 지표 정보를 담는 구조체
#[derive(Debug, Clone)]
pub struct TradeIndicators {
    rsi: f32,
    ma5: f32,
    ma20: f32,
    volume_ratio: f32,
}

impl Default for Futurx {
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
            for symbol in &uc::DEFAULT_ARR {
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
            alerts: VecDeque::with_capacity(5),
            auto_trading_enabled: false,
            last_trade_time: None,
            alert_sender,
            average_prices: HashMap::new(),
            momentum_enabled: false,
            momentum_buy_signals: BTreeMap::new(),
            momentum_sell_signals: BTreeMap::new(),
        }
    }
}

impl Futurx {
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
        let ma_controls = ma_controls(&self);

        let prediction_display = Container::new(Column::new().push(
            if let Some(alert) = self.alerts.front() {
                Text::new(&alert.message).color(match alert.alert_type {
                    AlertType::Buy => uc::BRIGH_GREEN,
                    AlertType::Sell => uc::BRIGHT_RED,
                    AlertType::Info => uc::BRIGHT_BLUE,
                    AlertType::Error => uc::BRIGHT_RED,
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
        //coin 가격 정보
        let coin_info = coin_info(&self);

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
            self.momentum_enabled,
            self.momentum_buy_signals.clone(),
            self.momentum_sell_signals.clone(),
        ))
        .width(iced::Fill)
        .height(Length::from(800));

        let auto_trading_toggle = auto_trading_toggle(&self);
        let account_info = account_info(&self);
        let order_buttons = order_buttons(&self);
        let current_position = current_position(&self);
        let right_side_bar = Column::new()
            .spacing(20)
            .padding(20)
            .push(auto_trading_toggle)
            .push(account_info)
            .push(order_buttons)
            // .push(Space::with_height(Length::Fill))
            .push(current_position);

        let left_side_bar = Column::new().spacing(20).padding(20).push(coin_info);

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
            Message::MarketBuy => market_buy(self),
            Message::MarketSell => market_sell(self),
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
                                println!("{}", ul::ORDER_FAIL);
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
                //알림제거
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
            Message::ToggleMomentum => {
                self.momentum_enabled = !self.momentum_enabled;
                if self.momentum_enabled {
                    let (buy_signals, sell_signals) = calculate_momentum_signals(
                        &self.candlesticks,
                        false,
                        &self.selected_candle_type,
                    );
                    self.momentum_buy_signals = buy_signals;
                    self.momentum_sell_signals = sell_signals;
                } else {
                    self.momentum_buy_signals.clear();
                    self.momentum_sell_signals.clear();
                }
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
                    if self.momentum_enabled {
                        let (buy_signals, sell_signals) = calculate_momentum_signals(
                            &self.candlesticks,
                            false,
                            &self.selected_candle_type,
                        ); // false 추가
                        self.momentum_buy_signals = buy_signals;
                        self.momentum_sell_signals = sell_signals;
                    }
                }
            }
            Message::ToggleMA5 => self.show_ma5 = !self.show_ma5,
            Message::ToggleMA10 => self.show_ma10 = !self.show_ma10,
            Message::ToggleMA20 => self.show_ma20 = !self.show_ma20,
            Message::ToggleMA200 => self.show_ma200 = !self.show_ma200,
            Message::SelectCandleType(candle_type) => {
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
                        if self.momentum_enabled {
                            let (buy_signals, sell_signals) = calculate_momentum_signals(
                                &self.candlesticks,
                                false,
                                &self.selected_candle_type,
                            );
                            self.momentum_buy_signals = buy_signals;
                            self.momentum_sell_signals = sell_signals;
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
                    info.price = price;
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

                            if self.momentum_enabled {
                                let (buy_signals, sell_signals) = calculate_momentum_signals(
                                    &self.candlesticks,
                                    false,
                                    &self.selected_candle_type,
                                );
                                self.momentum_buy_signals = buy_signals;
                                self.momentum_sell_signals = sell_signals;
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
                //knn
                if self.knn_enabled {
                    let (buy_signals, sell_signals) =
                        calculate_knn_signals(&self.candlesticks, true); // true로 실시간 표시
                    self.knn_buy_signals = buy_signals;
                    self.knn_sell_signals = sell_signals;
                }

                if self.momentum_enabled {
                    let (buy_signals, sell_signals) = calculate_momentum_signals(
                        &self.candlesticks,
                        true,
                        &self.selected_candle_type, // 이 부분이 이전에 빠져있었음
                    );
                    self.momentum_buy_signals = buy_signals;
                    self.momentum_sell_signals = sell_signals;
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

impl std::fmt::Display for CandleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandleType::Minute1 => write!(f, "1Minute"),
            CandleType::Minute3 => write!(f, "3Minute"), // 표시 텍스트 변경
            CandleType::Day => write!(f, "Day"),
        }
    }
}
trait VecDequeExt<T: 'static> {
    fn range(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = &(u64, T)>;
}

impl<T: 'static> VecDequeExt<T> for VecDeque<(u64, T)> {
    fn range(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = &(u64, T)> {
        self.iter().skip(range.start).take(range.end - range.start)
    }
}

fn main() -> iced::Result {
    dotenv().ok();

    iced::application("Candlestick Chart", Futurx::update, Futurx::view)
        .subscription(Futurx::subscription)
        .window_size(Size::new(uc::WINDOW_WIDTH, uc::WINDOW_HIGHT))
        .run()
}
