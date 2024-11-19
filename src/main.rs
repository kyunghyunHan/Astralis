use std::collections::BTreeMap;

use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::Stream;
use iced::futures::{channel::mpsc, SinkExt, StreamExt};
use iced::stream;
use iced::time::{self, Duration, Instant};
use iced::widget::canvas::Fill;
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
        column, container, pick_list, text, Column, Container, PickList, Space, Text,
    },
    Color, Element, Length, Pixels, Point, Rectangle, Size, Subscription,
};
use std::collections::HashMap;

use async_stream::stream;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
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
    SelectCandleType(CandleType), // 새로운 메시지 타입 추가
    Error,
    WebSocketInit(mpsc::Sender<String>),
    UpdatePrice(String, f64, f64),
}

struct RTarde {
    timer_enabled: bool,
    candlesticks: BTreeMap<u64, Candlestick>,
    selected_coin: String,
    selected_candle_type: CandleType, // 선택된 봉 타입 추가
    coin_list: HashMap<String, CoinInfo>,
    auto_scroll: bool,
    ws_sender: Option<mpsc::Sender<String>>,
}

#[derive(Debug, Clone)]
struct Candlestick {
    open: f32,
    close: f32,
    high: f32,
    low: f32,
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
    candle_type: CandleType, // 추가
}
impl Chart {
    fn new(candlesticks: BTreeMap<u64, Candlestick>, candle_type: CandleType) -> Self {
        let price_range = if candlesticks.is_empty() {
            Some((0.0, 100.0))
        } else {
            let (min, max) = candlesticks.values().fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

            let margin_percent = match max {
                p if p >= 10_000_000.0 => 0.001,
                p if p >= 1_000_000.0 => 0.002,
                p if p >= 100_000.0 => 0.005,
                p if p >= 10_000.0 => 0.01,
                _ => 0.02,
            };

            let margin = (max - min) * margin_percent;
            Some((min - margin, max + margin))
        };

        Self {
            candlesticks,
            state: ChartState {
                auto_scroll: true,
                ..ChartState::default()
            },
            price_range,
            candle_type, // 추가
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
        }
    }
}

enum WebSocketState {
    Disconnected,
    Connected(
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ),
}
#[derive(Debug, Clone)]
pub enum Event2 {
    Ready(mpsc::Sender<Input>),
    WorkFinished,
    Trade(UpbitWebSocketData),
}
#[derive(Debug, Clone, Deserialize)]
struct UpbitWebSocketData {
    ty: String,              // 타입
    code: String,            // 마켓 코드 (ex: KRW-BTC)
    timestamp: u64,          // 타임스탬프
    trade_price: f32,        // 체결가
    trade_volume: f32,       // 체결량
    ask_bid: String,         // 매수/매도 구분
    prev_closing_price: f32, // 전일 종가
    change: String,          // 전일 대비 (RISE, EVEN, FALL)
    change_price: f32,       // 부호 있는 변화액
    trade_date: String,      // 체결 날짜(UTC) (YYYYMMDD)
    trade_time: String,      // 체결 시각(UTC) (HHmmss)
    trade_timestamp: u64,    // 체결 타임스탬프
    opening_price: f32,      // 시가
    high_price: f32,         // 고가
    low_price: f32,          // 저가
    sequential_id: u64,      // 체결 번호
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
    prev_closing_price: f64,
    change: String,
    change_price: f64,
    sequential_id: u64,
    stream_type: String,
}
// UpbitTicker 구조체 추가
#[derive(Deserialize)]
struct UpbitTicker {
    code: String,
    trade_price: f64,
    change_rate: f64,
    #[serde(rename = "type")]
    ticker_type: String,
}
enum Input {
    Subscribe,
}

fn upbit_connection() -> impl Stream<Item = Message> {
    stream! {
        let url = Url::parse("wss://api.upbit.com/websocket/v1").unwrap();
        let (tx, mut rx) = mpsc::channel(100);

        yield Message::WebSocketInit(tx.clone());

        loop {
            if let Ok((mut ws_stream, _)) = connect_async(url.clone()).await {
                // 초기 구독 설정 (BTC)
                let initial_subscribe = json!([
                    {"ticket":"test"},
                    {
                        "type":"ticker",
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
                                    "type":"ticker",
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
                                ME::Binary(binary_content) => { // 변수명을 `binary_content`로 변경
                                    if let Ok(ticker) = serde_json::from_slice::<UpbitTicker>(&binary_content) {
                                        let symbol = ticker.code.replace("KRW-", "");
                                        yield Message::UpdatePrice(
                                            symbol,
                                            ticker.trade_price,
                                            ticker.change_rate * 100.0,
                                        );
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
        Subscription::run(upbit_connection)
    }
    pub fn view(&self) -> Element<Message> {
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
            let price_text = format!("현재가: {:.2} KRW", info.price);
            let change_text = format!("변동률: {:.2}%", info.change_percent);
            let change_color = if info.change_percent >= 0.0 {
                Color::from_rgb(0.8, 0.0, 0.0)
            } else {
                Color::from_rgb(0.0, 0.0, 0.8)
            };

            Row::new()
                .spacing(20)
                .push(Text::new(price_text))
                .push(Text::new(change_text).color(change_color))
        } else {
            Row::new().push(Text::new("로딩중..."))
        };

        // 수정된 부분: Chart::new()에 selected_candle_type 전달
        let canvas = Canvas::new(Chart::new(
            self.candlesticks.clone(),
            self.selected_candle_type.clone(),
        ))
        .width(iced::Fill)
        .height(Length::from(500));

        let left_side_bar = Column::new().push(current_coin_info);
        let right_side_bar = Column::new();

        Column::new()
            // .spacing(20)
            .push(
                Row::new()
                    // .spacing(20)
                    .push(coin_picker)
                    .push(candle_type_picker),
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

                // 현재 선택된 코인의 데이터만 처리
                let current_market = format!("KRW-{}", self.selected_coin);
                if trade_data.code != current_market {
                    return;
                }

                // 봉 타입에 따른 타임스탬프 계산
                let candle_timestamp = match self.selected_candle_type {
                    CandleType::Minute1 => timestamp - (timestamp % (60 * 1000)), // 1분
                    CandleType::Minute3 => timestamp - (timestamp % (3 * 60 * 1000)), // 3분
                    CandleType::Day => timestamp - (timestamp % (24 * 60 * 60 * 1000)), // 1일
                };

                let trade_price = trade_data.trade_price as f32;

                // 현재 가격 범위 계산 및 필터링
                if !self.candlesticks.is_empty() {
                    let (min, max) = self
                        .candlesticks
                        .values()
                        .fold((f32::MAX, f32::MIN), |acc, c| {
                            (acc.0.min(c.low), acc.1.max(c.high))
                        });

                    let valid_range = match self.selected_coin.as_str() {
                        "BTC" => (min * 0.7, max * 1.3),
                        "ETH" => (min * 0.6, max * 1.4),
                        _ => (min * 0.5, max * 1.5),
                    };

                    if trade_price < valid_range.0 || trade_price > valid_range.1 {
                        println!(
                            "Filtered price for {}: {:.2} (range: {:.2} ~ {:.2})",
                            self.selected_coin, trade_price, valid_range.0, valid_range.1
                        );
                        return;
                    }
                }

                // 캔들스틱 업데이트 또는 새로 추가
                if let Some(candlestick) = self.candlesticks.get_mut(&candle_timestamp) {
                    candlestick.high = candlestick.high.max(trade_price);
                    candlestick.low = candlestick.low.min(trade_price);
                    candlestick.close = trade_price;
                    println!(
                        "Updated candlestick at {}: O:{:.2} H:{:.2} L:{:.2} C:{:.2}",
                        candle_timestamp,
                        candlestick.open,
                        candlestick.high,
                        candlestick.low,
                        candlestick.close
                    );
                } else {
                    let new_candlestick = Candlestick {
                        open: trade_price,
                        high: trade_price,
                        low: trade_price,
                        close: trade_price,
                    };
                    println!(
                        "Created new candlestick at {}: {:.2}",
                        candle_timestamp, trade_price
                    );
                    self.candlesticks.insert(candle_timestamp, new_candlestick);

                    // 캔들 개수 제한 (최근 100개만 유지)
                    while self.candlesticks.len() > 100 {
                        if let Some(first_key) = self.candlesticks.keys().next().cloned() {
                            self.candlesticks.remove(&first_key);
                        }
                    }
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

        // 배경 그리기
        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            bounds.size(),
            Color::from_rgb(0.1, 0.1, 0.15),
        );

        // 가격 범위 계산
        let (mut min_price, mut max_price) = self
            .candlesticks
            .values()
            .fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

        // 마진 추가
        let margin = (max_price - min_price) * 0.1;
        min_price = (min_price - margin).max(0.0);
        max_price += margin;

        // 스케일링 계산
        let price_diff = (max_price - min_price).max(f32::EPSILON);
        let y_scale = ((bounds.height - 40.0) / price_diff).min(1e6);

        // 캔들스틱 크기 계산
        let candle_count = self.candlesticks.len() as f32;
        let available_width = bounds.width - 40.0;

        // 봉 타입에 따른 캔들스틱 크기 조정
        let fixed_candle_width = match self.candle_type {
            CandleType::Minute1 => (available_width / candle_count).min(8.0),
            CandleType::Minute3 => (available_width / candle_count).min(10.0), // 크기 조정
            CandleType::Day => (available_width / candle_count).min(15.0),
        };
        let body_width = fixed_candle_width * 0.8;

        // 그리드 라인
        for i in 0..=10 {
            let y = bounds.height * (i as f32 / 10.0);
            let price = min_price + (price_diff * (i as f32 / 10.0));

            // 가로 그리드 라인
            frame.stroke(
                &canvas::Path::new(|p| {
                    p.move_to(Point::new(30.0, y));
                    p.line_to(Point::new(bounds.width - 10.0, y));
                }),
                canvas::Stroke::default()
                    .with_color(Color::from_rgb(0.2, 0.2, 0.25))
                    .with_width(1.0),
            );

            // 가격 레이블
            frame.fill_text(canvas::Text {
                content: format!("{:.0}", price),
                position: Point::new(5.0, y - 5.0),
                color: Color::from_rgb(0.7, 0.7, 0.7),
                size: Pixels(10.0),
                ..canvas::Text::default()
            });
        }

        // 캔들스틱 그리기
        for (i, (timestamp, candlestick)) in self.candlesticks.iter().enumerate() {
            let x = 30.0 + (i as f32 * fixed_candle_width) + state.offset;

            // 화면 밖의 캔들은 건너뛰기
            if x < -fixed_candle_width || x > bounds.width {
                continue;
            }

            // 시간 레이블 (10개 간격으로)
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
                    position: Point::new(x, bounds.height - 5.0),
                    color: Color::from_rgb(0.7, 0.7, 0.7),
                    size: Pixels(10.0),
                    ..canvas::Text::default()
                });
            }

            let open_y = bounds.height - 20.0 - ((candlestick.open - min_price) * y_scale);
            let close_y = bounds.height - 20.0 - ((candlestick.close - min_price) * y_scale);
            let high_y = bounds.height - 20.0 - ((candlestick.high - min_price) * y_scale);
            let low_y = bounds.height - 20.0 - ((candlestick.low - min_price) * y_scale);

            let color = if candlestick.close >= candlestick.open {
                Color::from_rgb(0.8, 0.0, 0.0) // 상승 빨간색
            } else {
                Color::from_rgb(0.0, 0.0, 0.8) // 하락 파란색
            };

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
        }

        vec![frame.into_geometry()]
    }
}
fn fetch_daily_candles(
    market: &str,
) -> Result<BTreeMap<u64, Candlestick>, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    println!("Fetching candles for market: {}", market);

    rt.block_on(async {
        let url = format!(
            "https://api.upbit.com/v1/candles/days?market={}&count=200",
            market
        );

        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()).into());
        }

        let text = response.text().await?;
        let candles: Vec<UpbitCandle> = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error for {}: {} (Response: {})", market, e, text))?;

        if candles.is_empty() {
            return Err("No candles returned".into());
        }

        let result = candles
            .into_iter()
            .filter(|candle| {
                // 유효하지 않은 데이터 필터링
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
                        close: candle.trade_price,
                        high: candle.high_price,
                        low: candle.low_price,
                    },
                )
            })
            .collect();

        Ok(result)
    })
}
fn main() -> iced::Result {
    iced::application("Candlestick Chart", RTarde::update, RTarde::view)
        .subscription(RTarde::subscription)
        .window_size(Size::new(1980., 1080.))
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
            "https://api.upbit.com/v1/candles/minutes/1?market={}&count=100",
            market
        ),
        CandleType::Minute3 => format!(
            "https://api.upbit.com/v1/candles/minutes/3?market={}&count=100",
            market
        ),
        CandleType::Day => format!(
            "https://api.upbit.com/v1/candles/days?market={}&count=100",
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

    let result: BTreeMap<u64, Candlestick> = match candle_type {
        CandleType::Minute1 | CandleType::Minute3 => {
            let candles: Vec<UpbitMinuteCandle> = serde_json::from_str(&text).map_err(|e| {
                format!("Parse error for minute candles: {} (Response: {})", e, text)
            })?;

            println!("Parsed {} minute candles", candles.len());

            candles
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
                        },
                    )
                })
                .collect()
        }
        CandleType::Day => {
            let candles: Vec<UpbitCandle> = serde_json::from_str(&text)
                .map_err(|e| format!("Parse error for day candles: {} (Response: {})", e, text))?;

            println!("Parsed {} day candles", candles.len());

            candles
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
