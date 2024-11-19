#![allow(non_utf8_strings)]

use futures_util::Stream;
use iced::futures::{channel::mpsc, SinkExt, StreamExt};
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
        PickList, Space, Text,
    },
    Color, Element, Font, Length, Pixels, Point, Rectangle, Size, Subscription,
};
use std::collections::BTreeMap;
use std::collections::HashMap;

use async_stream::stream;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as ME}; // 여기에 Message를 임포트

#[derive(Debug, Clone)]
struct CoinInfo {
    symbol: String,
    name: String,
    price: f64,
    change_percent: f64,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddCandlestick((u64, UpbitTrade)),
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
}

struct RTarde {
    timer_enabled: bool,
    candlesticks: BTreeMap<u64, Candlestick>,
    selected_coin: String,
    selected_candle_type: CandleType, // 선택된 봉 타입 추가
    coin_list: HashMap<String, CoinInfo>,
    auto_scroll: bool,
    ws_sender: Option<mpsc::Sender<String>>,
    show_ma5: bool,   // 5일 이동평균선 표시 여부
    show_ma10: bool,  // 10일 이동평균선 표시 여부
    show_ma20: bool,  // 20일 이동평균선 표시 여부
    show_ma200: bool, // 200일 이동평균선 표시 여부
}

// Candlestick 구조체 업데이트
#[derive(Debug, Clone)]
struct Candlestick {
    open: f32,
    close: f32,
    high: f32,
    low: f32,
    volume: f32, // 거래량 필드 추가
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CandleType {
    Minute1,
    Minute3, // 2분봉을 3분봉으로 변경
    Day,
}
impl CandleType {
    fn as_str(&self) -> &'static str {
        match self {
            CandleType::Minute1 => "1Minute",
            CandleType::Minute3 => "3Minute", // 표시 텍스트 변경
            CandleType::Day => "Day",
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

#[derive(Default)]
struct ChartState {
    offset: f32,
    dragging: bool,
    drag_start: Point,
    last_offset: f32,
    auto_scroll: bool, // 자동 스크롤 상태 추가
}
use serde_json::json;
struct Chart {
    candlesticks: BTreeMap<u64, Candlestick>,
    state: ChartState,
    price_range: Option<(f32, f32)>,
    candle_type: CandleType,
    show_ma5: bool,
    show_ma10: bool,
    show_ma20: bool,
    show_ma200: bool,
    ma5_values: BTreeMap<u64, f32>,
    ma10_values: BTreeMap<u64, f32>,
    ma20_values: BTreeMap<u64, f32>,
    ma200_values: BTreeMap<u64, f32>,
}
impl Chart {
    fn new(
        candlesticks: BTreeMap<u64, Candlestick>,
        candle_type: CandleType,
        show_ma5: bool,
        show_ma10: bool,
        show_ma20: bool,
        show_ma200: bool,
    ) -> Self {
        let ma5_values = calculate_moving_average(&candlesticks, 5);
        let ma10_values = calculate_moving_average(&candlesticks, 10);
        let ma20_values = calculate_moving_average(&candlesticks, 20);
        let ma200_values = calculate_moving_average(&candlesticks, 200);

        let price_range = if candlesticks.is_empty() {
            Some((0.0, 100.0))
        } else {
            let (min, max) = candlesticks.values().fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

            // 이동평균선 값들도 고려하여 가격 범위 계산
            let ma_min = [&ma5_values, &ma10_values, &ma20_values, &ma200_values]
                .iter()
                .filter(|ma| !ma.is_empty())
                .flat_map(|ma| ma.values())
                .fold(min, |acc, &x| acc.min(x));

            let ma_max = [&ma5_values, &ma10_values, &ma20_values, &ma200_values]
                .iter()
                .filter(|ma| !ma.is_empty())
                .flat_map(|ma| ma.values())
                .fold(max, |acc, &x| acc.max(x));

            let margin = (ma_max - ma_min) * 0.1;
            Some((ma_min - margin, ma_max + margin))
        };

        Self {
            candlesticks,
            state: ChartState {
                auto_scroll: true,
                ..ChartState::default()
            },
            price_range,
            candle_type,
            show_ma5,
            show_ma10,
            show_ma20,
            show_ma200,
            ma5_values,
            ma10_values,
            ma20_values,
            ma200_values,
        }
    }
}

impl Default for RTarde {
    fn default() -> Self {
        let mut coin_list = HashMap::new();
        for symbol in &["BTC", "ETH", "XRP", "SOL", "DOT"] {
            coin_list.insert(
                symbol.to_string(),
                CoinInfo {
                    symbol: format!("KRW-{}", symbol),
                    name: symbol.to_string(),
                    price: 0.0,
                    change_percent: 0.0,
                },
            );
        }
        Self {
            candlesticks: fetch_candles("KRW-BTC", &CandleType::Day).unwrap_or_default(),
            timer_enabled: true,
            selected_coin: "BTC".to_string(),
            selected_candle_type: CandleType::Day,
            coin_list,
            auto_scroll: true,
            ws_sender: None,
            show_ma5: false,
            show_ma10: false,
            show_ma20: false,
            show_ma200: false,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct UpbitTrade {
    #[serde(rename = "type")]
    trade_type: String,
    code: String,
    timestamp: u64,
    trade_price: f64,
    trade_volume: f64,
    ask_bid: String,
    sequential_id: u64,
    prev_closing_price: f64,
    change: String,
    change_price: f64,
    stream_type: String,
}
fn calculate_moving_average(
    candlesticks: &BTreeMap<u64, Candlestick>,
    period: usize,
) -> BTreeMap<u64, f32> {
    let mut result = BTreeMap::new();
    if period == 0 || candlesticks.is_empty() {
        return result;
    }

    // 모든 캔들스틱의 timestamp와 종가를 벡터로 변환
    let data: Vec<(u64, f32)> = candlesticks
        .iter()
        .map(|(&timestamp, candle)| (timestamp, candle.close))
        .collect();

    // 이동 윈도우로 평균 계산
    for i in (period - 1)..data.len() {
        let sum: f32 = data[i + 1 - period..=i]
            .iter()
            .map(|(_, price)| price)
            .sum();
        let avg = sum / period as f32;
        result.insert(data[i].0, avg);
    }

    result
}

fn upbit_connection() -> impl Stream<Item = Message> {
    stream! {
        let url = Url::parse("wss://api.upbit.com/websocket/v1").unwrap();
        let (tx, mut rx) = mpsc::channel(100);

        yield Message::WebSocketInit(tx.clone());

        loop {
            if let Ok((mut ws_stream, _)) = connect_async(url.clone()).await {
                let initial_subscribe = json!([
                    {"ticket":"test"},
                    {
                        "type":"trade",    // 실시간 체결 데이터 구독
                        "codes":["KRW-BTC"],
                        "isOnlyRealtime": true
                    }
                ]).to_string();

                if let Err(_) = ws_stream.send(ME::Text(initial_subscribe)).await {
                    continue;
                }

                loop {
                    tokio::select! {
                        Some(new_coin) = rx.next() => {
                            let subscribe_message = json!([
                                {"ticket":"test"},
                                {
                                    "type":"trade",
                                    "codes":[format!("KRW-{}", new_coin)],
                                    "isOnlyRealtime": true
                                }
                            ]).to_string();

                            if let Err(_) = ws_stream.send(ME::Text(subscribe_message)).await {
                                break;
                            }
                            println!("Subscribed to {}", new_coin);
                        }
                        Some(Ok(message)) = ws_stream.next() => {
                            match message {
                                ME::Binary(binary_content) => {
                                    if let Ok(trade) = serde_json::from_slice::<UpbitTrade>(&binary_content) {
                                        // 가격 업데이트와 캔들스틱 업데이트를 동시에 전송
                                        let symbol = trade.code.replace("KRW-", "");
                                        yield Message::UpdatePrice(
                                            symbol.clone(),
                                            trade.trade_price,
                                            0.0,  // 변동률은 별도 계산 필요
                                        );
                                        yield Message::AddCandlestick((trade.timestamp, trade));
                                    }
                                }
                                _ => continue,
                            }
                        }
                        else => break,
                    }
                }
            }
            yield Message::Error;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
impl RTarde {
    pub fn subscription(&self) -> Subscription<Message> {
        // 실시간 데이터 구독
        Subscription::run(upbit_connection)
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
                ),
        )
        .padding(10);

        let coins: Vec<String> = self.coin_list.keys().cloned().collect();

        let coin_picker = pick_list(coins, Some(self.selected_coin.clone()), Message::SelectCoin)
            .width(Length::Fixed(150.0));

        // 봉 타입 선택을 위한 드롭다운
        let candle_types = vec![CandleType::Minute1, CandleType::Minute3, CandleType::Day];
        let candle_type_strings: Vec<String> = candle_types
            .iter()
            .map(|ct| ct.as_str().to_string())
            .collect();

        let candle_type_picker = pick_list(
            candle_type_strings,
            Some(self.selected_candle_type.as_str().to_string()),
            |s| {
                let candle_type = match s.as_str() {
                    "1Minute" => CandleType::Minute1, // 변경: "1분봉" -> "1Minute"
                    "3Minute" => CandleType::Minute3, // 변경: "3분봉" -> "3Minute"
                    "Day" => CandleType::Day,         // 변경: 매칭 문자열 통일
                    _ => CandleType::Day,
                };
                Message::SelectCandleType(candle_type)
            },
        )
        .width(Length::Fixed(100.0));

        let current_coin_info = if let Some(info) = self.coin_list.get(&self.selected_coin) {
            let font_bytes = include_bytes!("../assets/font/NanumGothic-Bold.ttf");
            let custom_font = Font::with_name("NotoSansCJK");

            // 가격 변화 화살표 (상승/하락)
            let price_direction = if info.change_percent >= 0.0 {
                "▲"
            } else {
                "▼"
            };

            let change_color = if info.change_percent >= 0.0 {
                Color::from_rgb(0.8, 0.0, 0.0)
            } else {
                Color::from_rgb(0.0, 0.0, 0.8)
            };

            Column::new()
                .spacing(10)
                .push(
                    // 코인 이름과 심볼
                    Container::new(
                        Column::new()
                            .push(
                                Text::new(&info.name)
                                    .size(28)
                                    .font(custom_font)
                                    .width(Length::Fill),
                            )
                            .push(
                                Text::new(&info.symbol)
                                    .size(14)
                                    .font(custom_font)
                                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
                            ),
                    )
                    .padding(10)
                    .width(Length::Fill),
                )
                .push(
                    // 가격 정보
                    Container::new(
                        Column::new()
                            .spacing(5)
                            .push(
                                // 현재가
                                Text::new(format!("{:.0} KRW", info.price))
                                    .size(32)
                                    .font(custom_font)
                                    .color(change_color),
                            )
                            .push(
                                // 변동률
                                Row::new()
                                    .spacing(5)
                                    .push(Text::new(price_direction).color(change_color))
                                    .push(
                                        Text::new(format!("{:.2}%", info.change_percent.abs()))
                                            .color(change_color),
                                    ),
                            ),
                    )
                    .padding(15)
                    // .style(iced::theme::Container::Custom(Box::new(|theme| {
                    //     iced::theme::Container {
                    //         background: Some(iced::Background::Color(Color::from_rgb(
                    //             0.95, 0.95, 0.95,
                    //         ))),
                    //         border_radius: 8.0.into(),
                    //         border_width: 1.0,
                    //         border_color: Color::from_rgb(0.9, 0.9, 0.9),
                    //         text_color: None,
                    //     }
                    // })))
                    .width(Length::Fill),
                )
                .push(
                    // 24시간 거래량 등 추가 정보를 위한 공간
                    Container::new(
                        Column::new()
                            .spacing(8)
                            .push(
                                Row::new()
                                    .spacing(5)
                                    .push(
                                        Text::new("24H 고가")
                                            .size(14)
                                            .color(Color::from_rgb(0.5, 0.5, 0.5)),
                                    )
                                    .push(
                                        Text::new(format!("{:.0} KRW", info.price * 1.1)) // 예시 데이터
                                            .size(14),
                                    ),
                            )
                            .push(
                                Row::new()
                                    .spacing(5)
                                    .push(
                                        Text::new("24H 저가")
                                            .size(14)
                                            .color(Color::from_rgb(0.5, 0.5, 0.5)),
                                    )
                                    .push(
                                        Text::new(format!("{:.0} KRW", info.price * 0.9)) // 예시 데이터
                                            .size(14),
                                    ),
                            ),
                    )
                    .padding(10)
                    .width(Length::Fill),
                )
        } else {
            Column::new().push(Text::new("로딩중..."))
        };

        // 수정된 부분: Chart::new()에 selected_candle_type 전달
        let canvas = Canvas::new(Chart::new(
            self.candlesticks.clone(),
            self.selected_candle_type.clone(),
            self.show_ma5,   // 5일 이동평균선 표시 여부
            self.show_ma10,  // 10일 이동평균선 표시 여부
            self.show_ma20,  // 20일 이동평균선 표시 여부
            self.show_ma200, // 200일 이동평균선 표시 여부
        ))
        .width(iced::Fill)
        .height(Length::from(500));

        let left_side_bar = Column::new()
            .spacing(20)
            .padding(20)
            .push(current_coin_info);
        let right_side_bar = Column::new()
            .spacing(20)
            .padding(20)
            .push(
                Container::new(Text::new("주문하기").size(24))
                    .width(Length::Fill)
                    .center_x(0),
            )
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("주문 유형").size(16))
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(
                                button(Text::new("시장가 매수"))
                                    // .style(iced::theme::Button::Primary)
                                    .width(Length::Fill),
                            )
                            .push(
                                button(Text::new("시장가 매도"))
                                    // .style(iced::theme::Button::Secondary)
                                    .width(Length::Fill),
                            ),
                    ),
            )
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("지정가 주문").size(16))
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(text_input("가격을 입력하세요...", ""))
                            .push(Text::new("KRW")),
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(text_input("수량을 입력하세요...", ""))
                            .push(Text::new(self.selected_coin.clone())),
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(
                                button(Text::new("지정가 매수"))
                                    // .style(iced::theme::Button::Primary)
                                    .width(Length::Fill),
                            )
                            .push(
                                button(Text::new("지정가 매도"))
                                    // .style(iced::theme::Button::Secondary)
                                    .width(Length::Fill),
                            ),
                    ),
            )
            .push(
                Column::new()
                    .spacing(10)
                    .push(Text::new("주문 비율").size(16))
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
            .push(
                Container::new(
                    Column::new()
                        .spacing(10)
                        .push(Text::new("보유자산").size(16))
                        .push(
                            Row::new()
                                .spacing(10)
                                .push(Text::new("KRW"))
                                .push(Text::new("1,000,000").size(16)),
                        )
                        .push(
                            Row::new()
                                .spacing(10)
                                .push(Text::new(&self.selected_coin))
                                .push(Text::new("0.0").size(16)),
                        ),
                ), // .style(iced::theme::Container::Box),
            );
        Column::new()
            // .spacing(20)
            .push(
                Row::new()
                    // .spacing(20)
                    .push(coin_picker)
                    .push(candle_type_picker)
                    .push(ma_controls),
            )
            .push(
                Row::new()
                    // .spacing(10)
                    .push(container(left_side_bar).width(FillPortion(1)))
                    .push(container(canvas).width(FillPortion(3)))
                    .push(container(right_side_bar).width(FillPortion(1))),
            )
            .into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToggleMA5 => self.show_ma5 = !self.show_ma5,
            Message::ToggleMA10 => self.show_ma10 = !self.show_ma10,
            Message::ToggleMA20 => self.show_ma20 = !self.show_ma20,
            Message::ToggleMA200 => self.show_ma200 = !self.show_ma200,
            Message::SelectCandleType(candle_type) => {
                println!("Changing candle type to: {}", candle_type);
                self.selected_candle_type = candle_type.clone();

                // 캔들스틱 데이터 새로 불러오기
                let market = format!("KRW-{}", self.selected_coin);
                println!(
                    "Fetching new candles for market {} with type {}",
                    market, candle_type
                );

                match fetch_candles(&market, &candle_type) {
                    Ok(candles) => {
                        println!(
                            "Successfully fetched {} candles for {}",
                            candles.len(),
                            candle_type
                        );
                        self.candlesticks = candles;
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
                    info.change_percent = change_rate;
                    println!("Updated price for {}: {} ({}%)", symbol, price, change_rate);
                    // 디버그용
                }
            }
            Message::WebSocketInit(sender) => {
                self.ws_sender = Some(sender);
            }

            Message::SelectCoin(symbol) => {
                println!("Switching to coin: {}", symbol);
                self.selected_coin = symbol.clone();
                self.candlesticks.clear();

                // 새로운 코인의 캔들스틱 데이터 불러오기
                match fetch_candles(&format!("KRW-{}", symbol), &self.selected_candle_type) {
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
                    info.change_percent = change;
                }
            }
            Message::AddCandlestick(trade) => {
                let (timestamp, trade_data) = trade;
                let current_market = format!("KRW-{}", self.selected_coin);

                if trade_data.code != current_market {
                    return;
                }

                let current_timestamp = timestamp;
                let candle_timestamp = match self.selected_candle_type {
                    CandleType::Minute1 => current_timestamp - (current_timestamp % (60 * 1000)),
                    CandleType::Minute3 => {
                        current_timestamp - (current_timestamp % (3 * 60 * 1000))
                    }
                    CandleType::Day => {
                        current_timestamp - (current_timestamp % (24 * 60 * 60 * 1000))
                    }
                };

                let trade_price = trade_data.trade_price as f32;

                // 최신 캔들스틱과 비교
                let latest_timestamp = self.candlesticks.keys().next_back().cloned();

                if let Some(latest_ts) = latest_timestamp {
                    if candle_timestamp > latest_ts {
                        // 새로운 캔들스틱 시작
                        let new_candlestick = Candlestick {
                            open: trade_price,
                            high: trade_price,
                            low: trade_price,
                            close: trade_price,
                            volume: trade_data.trade_volume as f32, // 초기 거래량 설정
                        };
                        self.candlesticks.insert(candle_timestamp, new_candlestick);
                        println!(
                            "New candlestick at: {} price: {}, volume: {}",
                            candle_timestamp, trade_price, trade_data.trade_volume
                        );

                        // 오래된 캔들 제거
                        while self.candlesticks.len() > 100 {
                            if let Some(first_key) = self.candlesticks.keys().next().cloned() {
                                self.candlesticks.remove(&first_key);
                            }
                        }
                    } else if candle_timestamp == latest_ts {
                        // 현재 캔들스틱 업데이트
                        if let Some(current_candle) = self.candlesticks.get_mut(&candle_timestamp) {
                            current_candle.high = current_candle.high.max(trade_price);
                            current_candle.low = current_candle.low.min(trade_price);
                            current_candle.close = trade_price;
                            current_candle.volume += trade_data.trade_volume as f32; // 거래량 누적
                            println!(
                                "Updated candlestick: close {}, volume {}",
                                trade_price, current_candle.volume
                            );
                        }
                    }
                } else {
                    // 첫 캔들스틱 생성
                    let new_candlestick = Candlestick {
                        open: trade_price,
                        high: trade_price,
                        low: trade_price,
                        close: trade_price,
                        volume: trade_data.trade_volume as f32, // 초기 거래량 설정
                    };
                    self.candlesticks.insert(candle_timestamp, new_candlestick);
                }

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
}

impl<Message> Program<Message> for Chart {
    type State = ChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<Message>) {
        let cursor_position = if let Some(position) = cursor.position() {
            position
        } else {
            return (event::Status::Ignored, None);
        };

        match event {
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    state.dragging = true;
                    state.drag_start = cursor_position;
                    state.last_offset = state.offset;
                    state.auto_scroll = false;
                    (event::Status::Captured, None)
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    state.dragging = false;
                    (event::Status::Captured, None)
                }
                mouse::Event::CursorMoved { .. } => {
                    if state.dragging {
                        let delta_x = cursor_position.x - state.drag_start.x;
                        state.offset = state.last_offset + delta_x;
                        (event::Status::Captured, None)
                    } else {
                        (event::Status::Ignored, None)
                    }
                }
                _ => (event::Status::Ignored, None),
            },
            _ => (event::Status::Ignored, None),
        }
    }
    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if self.candlesticks.is_empty() {
            return vec![frame.into_geometry()];
        }
        let (mut min_price, mut max_price) = self
            .candlesticks
            .values()
            .fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

        // 이동평균선의 최소/최대값도 고려
        let ma_values = [
            (self.show_ma5, &self.ma5_values),
            (self.show_ma10, &self.ma10_values),
            (self.show_ma20, &self.ma20_values),
            (self.show_ma200, &self.ma200_values),
        ];

        for (show, values) in ma_values.iter() {
            if *show && !values.is_empty() {
                for &ma_value in values.values() {
                    min_price = min_price.min(ma_value);
                    max_price = max_price.max(ma_value);
                }
            }
        }

        // 여유 공간 추가
        let price_margin = (max_price - min_price) * 0.1;
        min_price -= price_margin;
        max_price += price_margin;

        // 이동평균선 그리기
        let ma_lines = [
            (
                self.show_ma5,
                &self.ma5_values,
                Color::from_rgb(1.0, 1.0, 0.0),
            ), // 노란색
            (
                self.show_ma10,
                &self.ma10_values,
                Color::from_rgb(0.0, 1.0, 0.0),
            ), // 녹색
            (
                self.show_ma20,
                &self.ma20_values,
                Color::from_rgb(1.0, 0.0, 1.0),
            ), // 보라색
            (
                self.show_ma200,
                &self.ma200_values,
                Color::from_rgb(0.0, 1.0, 1.0),
            ), // 하늘색
        ];

        // 여백 설정
        let left_margin = 50.0;
        let right_margin = 20.0;
        let top_margin = 20.0;
        let bottom_margin = 50.0;

        // 거래량 차트의 높이 설정
        let volume_height = 100.0;
        let price_chart_height = bounds.height - volume_height - bottom_margin - top_margin - 20.0; // 차트 사이 간격 추가
                                                                                                    // 스케일링 계산
        let price_diff = (max_price - min_price).max(f32::EPSILON);
        let y_scale = (price_chart_height / price_diff).min(1e6);

        // 배경 그리기
        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            bounds.size(),
            Color::from_rgb(0.1, 0.1, 0.15),
        );

        // 가격 및 거래량 범위 계산
        let (mut min_price, mut max_price) = self
            .candlesticks
            .values()
            .fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

        let max_volume = self
            .candlesticks
            .values()
            .map(|c| c.volume)
            .fold(0.0, f32::max);

        // 마진 추가
        let margin = (max_price - min_price) * 0.1;
        min_price = (min_price - margin).max(0.0);
        max_price += margin;

        // 스케일링 계산
        let price_diff = (max_price - min_price).max(f32::EPSILON);
        let y_scale = (price_chart_height / price_diff).min(1e6);
        let volume_scale = (volume_height / max_volume).min(1e6);

        // 캔들스틱 크기 계산
        let candle_count = self.candlesticks.len() as f32;
        let available_width = bounds.width - left_margin - right_margin;
        let fixed_candle_width = match self.candle_type {
            CandleType::Minute1 => (available_width / candle_count).min(8.0),
            CandleType::Minute3 => (available_width / candle_count).min(10.0),
            CandleType::Day => (available_width / candle_count).min(15.0),
        };
        let body_width = fixed_candle_width * 0.8;

        // 가격 차트 그리드 라인
        for i in 0..=10 {
            let y = top_margin + (price_chart_height * (i as f32 / 10.0));
            let price = max_price - (price_diff * (i as f32 / 10.0));

            frame.stroke(
                &canvas::Path::new(|p| {
                    p.move_to(Point::new(left_margin, y));
                    p.line_to(Point::new(bounds.width - right_margin, y));
                }),
                canvas::Stroke::default()
                    .with_color(Color::from_rgb(0.2, 0.2, 0.25))
                    .with_width(1.0),
            );

            frame.fill_text(canvas::Text {
                content: format!("{:.0}", price),
                position: Point::new(5.0, y - 5.0),
                color: Color::from_rgb(0.7, 0.7, 0.7),
                size: Pixels(10.0),
                ..canvas::Text::default()
            });
        }

        // 거래량 차트 영역 구분선
        let volume_area_start = top_margin + price_chart_height + 10.0;
        let volume_area_end = volume_area_start + volume_height;
        frame.stroke(
            &canvas::Path::new(|p| {
                p.move_to(Point::new(left_margin, volume_area_start));
                p.line_to(Point::new(bounds.width - right_margin, volume_area_start));
            }),
            canvas::Stroke::default()
                .with_color(Color::from_rgb(0.3, 0.3, 0.3))
                .with_width(1.0),
        );

        // 거래량 차트 그리드 라인
        for i in 0..=2 {
            let y = volume_area_end - (volume_height * (i as f32 / 2.0)); // 위치를 뒤집음
            let volume = max_volume * (i as f32 / 2.0);

            frame.stroke(
                &canvas::Path::new(|p| {
                    p.move_to(Point::new(left_margin, y));
                    p.line_to(Point::new(bounds.width - right_margin, y));
                }),
                canvas::Stroke::default()
                    .with_color(Color::from_rgb(0.2, 0.2, 0.25))
                    .with_width(1.0),
            );

            frame.fill_text(canvas::Text {
                content: format!("{:.0}", volume),
                position: Point::new(5.0, y - 5.0),
                color: Color::from_rgb(0.7, 0.7, 0.7),
                size: Pixels(10.0),
                ..canvas::Text::default()
            });
        }
        // 캔들스틱과 거래량 바 그리기
        for (i, (timestamp, candlestick)) in self.candlesticks.iter().enumerate() {
            let x = left_margin + (i as f32 * fixed_candle_width) + state.offset;

            if x < -fixed_candle_width || x > bounds.width {
                continue;
            }

            // 시간 레이블
            if i % 10 == 0 {
                let time_str = match self.candle_type {
                    CandleType::Minute1 | CandleType::Minute3 => {
                        let dt = chrono::DateTime::from_timestamp((*timestamp / 1000) as i64, 0)
                            .unwrap_or_default()
                            .with_timezone(&chrono::Local);
                        dt.format("%H:%M").to_string()
                    }
                    CandleType::Day => {
                        let dt = chrono::DateTime::from_timestamp((*timestamp / 1000) as i64, 0)
                            .unwrap_or_default()
                            .with_timezone(&chrono::Local);
                        dt.format("%m/%d").to_string()
                    }
                };

                frame.fill_text(canvas::Text {
                    content: time_str,
                    position: Point::new(x - 15.0, bounds.height - bottom_margin + 15.0),
                    color: Color::from_rgb(0.7, 0.7, 0.7),
                    size: Pixels(10.0),
                    ..canvas::Text::default()
                });
            }

            let color = if candlestick.close >= candlestick.open {
                Color::from_rgb(0.8, 0.0, 0.0)
            } else {
                Color::from_rgb(0.0, 0.0, 0.8)
            };

            // 캔들스틱 그리기
            let open_y = top_margin + ((max_price - candlestick.open) * y_scale);
            let close_y = top_margin + ((max_price - candlestick.close) * y_scale);
            let high_y = top_margin + ((max_price - candlestick.high) * y_scale);
            let low_y = top_margin + ((max_price - candlestick.low) * y_scale);

            // 심지
            let center_x = x + (body_width / 2.0);
            frame.stroke(
                &canvas::Path::new(|builder| {
                    builder.move_to(Point::new(center_x, high_y));
                    builder.line_to(Point::new(center_x, low_y));
                }),
                canvas::Stroke::default().with_color(color).with_width(1.0),
            );

            // 캔들 몸통
            let body_height = (close_y - open_y).abs().max(1.0);
            let body_y = close_y.min(open_y);
            frame.fill_rectangle(
                Point::new(x, body_y),
                Size::new(body_width, body_height),
                color,
            );

            // 거래량 바 그리기
            // 거래량 바 그리기 부분
            let volume_height = candlestick.volume * volume_scale;
            let volume_color = if candlestick.close >= candlestick.open {
                Color::from_rgba(0.8, 0.0, 0.0, 0.5)
            } else {
                Color::from_rgba(0.0, 0.0, 0.8, 0.5)
            };

            // 거래량 바의 시작점을 volume_area_end에서 시작하여 위로 그리기
            frame.fill_rectangle(
                Point::new(x, volume_area_end),
                Size::new(body_width, -volume_height), // 음수 높이로 위 방향으로 그리기
                volume_color,
            );
        }
        for (show, values, color) in ma_lines.iter() {
            if *show && !values.is_empty() {
                let mut path_builder = canvas::path::Builder::new();
                let mut first = true;

                // timestamp를 정렬된 순서로 처리
                let timestamps: Vec<_> = self.candlesticks.keys().collect();

                for (i, &timestamp) in timestamps.iter().enumerate() {
                    if let Some(&ma_price) = values.get(timestamp) {
                        let x = left_margin + (i as f32 * fixed_candle_width) + state.offset;
                        let y = top_margin + ((max_price - ma_price) * y_scale);

                        if first {
                            path_builder.move_to(Point::new(x, y));
                            first = false;
                        } else {
                            path_builder.line_to(Point::new(x, y));
                        }
                    }
                }

                let path = path_builder.build();

                frame.stroke(
                    &path,
                    canvas::Stroke::default().with_color(*color).with_width(1.5),
                );
            }
        }
        vec![frame.into_geometry()]
    }
}

fn main() -> iced::Result {
    let font_bytes = include_bytes!("../assets/font/NanumGothic-Bold.ttf");

    iced::application("Candlestick Chart", RTarde::update, RTarde::view)
        .subscription(RTarde::subscription)
        .window_size(Size::new(1980., 1080.))
        .font(font_bytes)
        .run()
}
#[derive(Debug, Deserialize, Clone)]
struct UpbitCandle {
    candle_acc_trade_price: f32,
    candle_acc_trade_volume: f32,
    candle_date_time_kst: String,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    change_price: f32,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    change_rate: f32,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    high_price: f32,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    low_price: f32,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    opening_price: f32,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    prev_closing_price: f32,
    timestamp: u64,
    #[serde(deserialize_with = "deserialize_f32_or_null")]
    trade_price: f32,
    unit: Option<i32>, // 분봉 데이터일 경우 분 단위
}
#[derive(Debug, Deserialize, Clone)]
struct UpbitMinuteCandle {
    market: String,
    candle_date_time_utc: String,
    candle_date_time_kst: String,
    opening_price: f64,
    high_price: f64,
    low_price: f64,
    trade_price: f64,
    timestamp: u64,
    candle_acc_trade_price: f64,
    candle_acc_trade_volume: f64,
    unit: i32, // 분봉 단위(1, 2, ...)
}

fn deserialize_f32_or_null<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNull {
        String(String),
        Float(f32),
        Null,
    }

    match StringOrNull::deserialize(deserializer)? {
        StringOrNull::String(s) => s.parse::<f32>().map_err(Error::custom),
        StringOrNull::Float(f) => Ok(f),
        StringOrNull::Null => Ok(0.0), // null인 경우 0.0으로 처리
    }
}
async fn fetch_candles_async(
    market: &str,
    candle_type: &CandleType,
) -> Result<BTreeMap<u64, Candlestick>, Box<dyn std::error::Error>> {
    let url = match candle_type {
        CandleType::Minute1 => format!(
            "https://api.upbit.com/v1/candles/minutes/1?market={}&count=1000",
            market
        ),
        CandleType::Minute3 => format!(
            "https://api.upbit.com/v1/candles/minutes/3?market={}&count=1000",
            market
        ),
        CandleType::Day => format!(
            "https://api.upbit.com/v1/candles/days?market={}&count=1000",
            market
        ),
    };

    println!("Requesting URL: {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        let error_msg = format!("API error: {}", response.status());
        println!("{}", error_msg);
        return Err(error_msg.into());
    }

    let text = response.text().await?;
    println!("Response text sample: {:.200}...", text);

    let mut result: BTreeMap<u64, Candlestick> = match candle_type {
        CandleType::Minute1 | CandleType::Minute3 => {
            let candles: Vec<UpbitMinuteCandle> = serde_json::from_str(&text)?;
            let mut sorted_candles = candles;
            sorted_candles.sort_by_key(|c| c.timestamp);

            sorted_candles
                .into_iter()
                .filter(|candle| {
                    candle.opening_price > 0.0
                        && candle.high_price > 0.0
                        && candle.low_price > 0.0
                        && candle.trade_price > 0.0
                })
                .map(|candle| {
                    (
                        candle.timestamp,
                        Candlestick {
                            open: candle.opening_price as f32,
                            high: candle.high_price as f32,
                            low: candle.low_price as f32,
                            close: candle.trade_price as f32,
                            volume: candle.candle_acc_trade_volume as f32, // 거래량 추가
                        },
                    )
                })
                .collect()
        }
        CandleType::Day => {
            let candles: Vec<UpbitCandle> = serde_json::from_str(&text)?;
            let mut sorted_candles = candles;
            sorted_candles.sort_by_key(|c| c.timestamp);

            sorted_candles
                .into_iter()
                .filter(|candle| {
                    candle.opening_price > 0.0
                        && candle.high_price > 0.0
                        && candle.low_price > 0.0
                        && candle.trade_price > 0.0
                })
                .map(|candle| {
                    (
                        candle.timestamp,
                        Candlestick {
                            open: candle.opening_price,
                            high: candle.high_price,
                            low: candle.low_price,
                            close: candle.trade_price,
                            volume: candle.candle_acc_trade_volume, // 거래량 추가
                        },
                    )
                })
                .collect()
        }
    };

    println!("Final processed candles count: {}", result.len());

    if result.is_empty() {
        Err("No valid candles returned".into())
    } else {
        Ok(result)
    }
}

fn fetch_candles(
    market: &str,
    candle_type: &CandleType,
) -> Result<BTreeMap<u64, Candlestick>, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    println!("Fetching {:?} candles for market: {}", candle_type, market);
    rt.block_on(fetch_candles_async(market, candle_type))
}
