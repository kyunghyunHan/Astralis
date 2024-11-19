use std::collections::BTreeMap;

use futures_util::stream::SplitSink;
use futures_util::stream::SplitStream;
use futures_util::Stream;
use iced::futures::{channel::mpsc, SinkExt, StreamExt};
use iced::stream;
use iced::time::{self, Duration, Instant};
use iced::widget::Row;
use iced::{
    mouse,
    widget::{
        button, canvas,
        canvas::{
            event::{self, Event},
            Canvas, Program,
        },
        column, container, pick_list, text, Column, Container, PickList, Text,
    },
    Color, Element, Length, Point, Rectangle, Size, Subscription,
};
use std::collections::HashMap;

use async_stream::stream;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as ME}; // 여기에 Message를 임포트
enum State {
    Disconnected,
    Connected(WebSocketStream<MaybeTlsStream<TcpStream>>),
}

#[derive(Debug, Clone)]
struct CoinInfo {
    symbol: String,
    name: String,
    price: f64,
    change_percent: f64,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddCandlestick((u64, UpbitTrade)), // UpbitTrade 데이터 포함
    RemoveCandlestick,
    SelectCoin(String),                // 코인 선택 메시지 추가
    UpdateCoinPrice(String, f64, f64), // 코인 가격 업데이트
    Error,
    WebSocketInit(mpsc::Sender<String>), // 추가
    UpdatePrice(String, f64, f64),       // (코인심볼, 현재가격, 변동률)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fruit {
    Apple,
    Orange,
    Strawberry,
    Tomato,
}
struct Counter {
    timer_enabled: bool,
    candlesticks: BTreeMap<u64, Candlestick>,
    selected_coin: String,                // 현재 선택된 코인
    coin_list: HashMap<String, CoinInfo>, // 코인 목록
    auto_scroll: bool,
    ws_sender: Option<mpsc::Sender<String>>, // 추가
}

#[derive(Debug, Clone)]
struct Candlestick {
    open: f32,
    close: f32,
    high: f32,
    low: f32,
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
    price_range: Option<(f32, f32)>, // min, max 가격 범위
}
#[derive(Debug, Clone)]
struct CoinPriceRange {
    min_price: f32,
    max_price: f32,
}
impl Chart {
    fn new(candlesticks: BTreeMap<u64, Candlestick>) -> Self {
        // 기본값으로 현재 데이터의 최저/최고값 사용
        let price_range = if candlesticks.is_empty() {
            Some((0.0, 100.0)) // Option으로 감싸기
        } else {
            let (min, max) = candlesticks.values().fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

            // 가격대별로 적절한 범위 설정
            let margin_percent = match max {
                p if p >= 10_000_000.0 => 0.001, // BTC처럼 고가 코인은 0.1% 마진
                p if p >= 1_000_000.0 => 0.002,  // ETH처럼 중고가 코인은 0.2% 마진
                p if p >= 100_000.0 => 0.005,    // SOL처럼 중가 코인은 0.5% 마진
                p if p >= 10_000.0 => 0.01,      // DOT처럼 중저가 코인은 1% 마진
                _ => 0.02,                       // XRP처럼 저가 코인은 2% 마진
            };

            let margin = (max - min) * margin_percent;
            Some((min - margin, max + margin)) // Option으로 감싸기
        };

        Self {
            candlesticks,
            state: ChartState {
                auto_scroll: true,
                ..ChartState::default()
            },
            price_range,
        }
    }
}
impl std::fmt::Display for Fruit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Apple => "Apple",
            Self::Orange => "Orange",
            Self::Strawberry => "Strawberry",
            Self::Tomato => "Tomato",
        })
    }
}
impl Default for Counter {
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
            candlesticks: fetch_daily_candles("KRW-BTC").unwrap_or_default(),
            timer_enabled: true,
            selected_coin: "BTC".to_string(),
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
impl Counter {
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(upbit_connection)
    }
    pub fn view(&self) -> Element<Message> {
        // 드롭다운용 코인 리스트 생성
        let coins: Vec<String> = self.coin_list.keys().cloned().collect();

        // 드롭다운 생성
        let coin_picker = pick_list(
            coins, // & 제거
            Some(self.selected_coin.clone()),
            Message::SelectCoin,
        )
        .width(Length::Fixed(150.0));

        // 현재 선택된 코인의 정보 표시
        let current_coin_info = if let Some(info) = self.coin_list.get(&self.selected_coin) {
            let price_text = format!("현재가: {:.2} KRW", info.price);
            let change_text = format!("변동률: {:.2}%", info.change_percent);
            let change_color = if info.change_percent >= 0.0 {
                Color::from_rgb(0.8, 0.0, 0.0) // 상승 시 빨간색
            } else {
                Color::from_rgb(0.0, 0.0, 0.8) // 하락 시 파란색
            };

            Row::new()
                .spacing(20)
                .push(Text::new(price_text))
                .push(Text::new(change_text).color(change_color)) // .style -> .color
        } else {
            Row::new().push(Text::new("로딩중..."))
        };

        let canvas = Canvas::new(Chart::new(self.candlesticks.clone()))
            .width(Length::Fill)
            .height(Length::from(500));

        Column::new()
            .spacing(20)
            .push(
                Row::new()
                    .spacing(20)
                    .push(coin_picker)
                    .push(current_coin_info),
            )
            .push(
                container(canvas)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(20),
            )
            .into()
    }
    pub fn update(&mut self, message: Message) {
        match message {
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
                self.selected_coin = symbol.clone();
                println!("Switching to coin: {}", symbol);

                // 캔들스틱 데이터 완전히 초기화
                self.candlesticks.clear();

                // 새로운 코인의 캔들스틱 데이터 불러오기
                match fetch_daily_candles(&format!("KRW-{}", symbol)) {
                    Ok(candles) => {
                        if candles.is_empty() {
                            println!("Warning: No candles received for {}", symbol);
                        } else {
                            let (min, max) =
                                candles.values().fold((f32::MAX, f32::MIN), |acc, c| {
                                    (acc.0.min(c.low), acc.1.max(c.high))
                                });
                            println!(
                                "Successfully loaded {} candles for {}. Price range: {:.2} ~ {:.2}",
                                candles.len(),
                                symbol,
                                min,
                                max
                            );
                            self.candlesticks = candles;
                        }
                    }
                    Err(e) => {
                        println!("Error fetching candles for {}: {:?}", symbol, e);
                    }
                }

                // WebSocket 구독 갱신
                if let Some(sender) = &self.ws_sender {
                    if let Err(e) = sender.clone().try_send(symbol.clone()) {
                        println!("Error sending WebSocket subscription: {:?}", e);
                    } else {
                        println!("Successfully subscribed to WebSocket for {}", symbol);
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

                let day_timestamp = timestamp - (timestamp % (24 * 60 * 60 * 1000));
                let trade_price = trade_data.trade_price as f32;

                // 현재 가격 범위 계산
                if !self.candlesticks.is_empty() {
                    let (min, max) = self
                        .candlesticks
                        .values()
                        .fold((f32::MAX, f32::MIN), |acc, c| {
                            (acc.0.min(c.low), acc.1.max(c.high))
                        });

                    // 가격 범위 체크 - 코인별로 다른 기준 적용
                    let valid_range = match self.selected_coin.as_str() {
                        "BTC" => (min * 0.7, max * 1.3), // BTC: ±30%
                        "ETH" => (min * 0.6, max * 1.4), // ETH: ±40%
                        _ => (min * 0.5, max * 1.5),     // 다른 코인: ±50%
                    };

                    if trade_price < valid_range.0 || trade_price > valid_range.1 {
                        println!(
                            "Filtered price for {}: {:.2} (range: {:.2} ~ {:.2})",
                            self.selected_coin, trade_price, valid_range.0, valid_range.1
                        );
                        return;
                    }

                    println!("Added price for {}: {:.2}", self.selected_coin, trade_price);
                }

                // 캔들스틱 업데이트 또는 새로 추가
                if let Some(candlestick) = self.candlesticks.get_mut(&day_timestamp) {
                    candlestick.high = candlestick.high.max(trade_price);
                    candlestick.low = candlestick.low.min(trade_price);
                    candlestick.close = trade_price;
                } else {
                    let new_candlestick = Candlestick {
                        open: trade_price,
                        high: trade_price,
                        low: trade_price,
                        close: trade_price,
                    };
                    self.candlesticks.insert(day_timestamp, new_candlestick);
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
        // 배경색 설정
        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            bounds.size(),
            Color::from_rgb(0.1, 0.1, 0.15),
        );

        // 그리드 라인 추가
        for i in 0..10 {
            let y = bounds.height * (i as f32 / 10.0);
            frame.stroke(
                &canvas::Path::new(|p| {
                    p.move_to(Point::new(0.0, y));
                    p.line_to(Point::new(bounds.width, y));
                }),
                canvas::Stroke::default()
                    .with_color(Color::from_rgb(0.2, 0.2, 0.25))
                    .with_width(1.0),
            );
        }

        // 캔들스틱 색상 수정
        let up_color = Color::from_rgb(0.0, 0.8, 0.4); // 초록색
        let down_color = Color::from_rgb(0.8, 0.2, 0.2); // 빨간색
                                                         // 가격 범위 계산을 더 안전하게
        let (mut min_price, mut max_price) =
            self.candlesticks
                .values()
                .fold((f32::MAX, f32::MIN), |acc, c| {
                    (
                        acc.0.min(c.low).max(0.0), // 음수 방지
                        acc.1.max(c.high),
                    )
                });

        // min과 max가 같으면 약간의 차이를 줌
        if (max_price - min_price).abs() < f32::EPSILON {
            max_price += max_price * 0.001;
            min_price -= min_price * 0.001;
        }

        // 마진 추가
        let margin = (max_price - min_price) * 0.1;
        min_price = (min_price - margin).max(0.0); // 음수 방지
        max_price += margin;

        // y_scale 계산을 더 안전하게
        let price_diff = (max_price - min_price).max(f32::EPSILON); // 0으로 나누기 방지
        let y_scale = ((bounds.height - 40.0) / price_diff).min(1e6); // 너무 큰 값 방지

        let candle_count = self.candlesticks.len() as f32;
        let available_width = bounds.width - 40.0;
        let fixed_candle_width = (available_width / candle_count).min(20.0);
        let body_width = fixed_candle_width * 0.8;

        // 스케일링된 y 좌표 계산 함수
        let scale_price = |price: f32| -> f32 {
            let scaled = bounds.height - 20.0 - ((price - min_price) * y_scale);
            scaled.min(bounds.height).max(0.0) // 범위 제한
        };

        // 캔들스틱 그리기
        for (i, (_, candlestick)) in self.candlesticks.iter().enumerate() {
            let x = 20.0 + (i as f32 * fixed_candle_width) + state.offset;

            // 화면 밖의 캔들은 건너뛰기
            if x < -fixed_candle_width || x > bounds.width {
                continue;
            }

            let open_y = scale_price(candlestick.open);
            let close_y = scale_price(candlestick.close);
            let high_y = scale_price(candlestick.high);
            let low_y = scale_price(candlestick.low);

            // 좌표가 유효한지 확인
            if !open_y.is_finite()
                || !close_y.is_finite()
                || !high_y.is_finite()
                || !low_y.is_finite()
            {
                continue;
            }

            let color = if candlestick.close >= candlestick.open {
                Color::from_rgb(0.8, 0.0, 0.0)
            } else {
                Color::from_rgb(0.0, 0.0, 0.8)
            };

            // 심지 그리기
            let center_x = x + (body_width / 2.0);
            frame.stroke(
                &canvas::Path::new(|builder| {
                    builder.move_to(Point::new(center_x, high_y));
                    builder.line_to(Point::new(center_x, low_y));
                }),
                canvas::Stroke::default().with_color(color).with_width(1.0),
            );

            // 몸통 그리기
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
    iced::application("Candlestick Chart", Counter::update, Counter::view)
        .subscription(Counter::subscription)
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
