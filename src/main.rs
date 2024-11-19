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
                loop {
                    tokio::select! {
                        Some(new_coin) = rx.next() => {
                            let subscribe_message = json!([
                                {"ticket":"test"},
                                {"type":"ticker","codes":[format!("KRW-{}", new_coin)]},
                                {"type":"trade","codes":[format!("KRW-{}", new_coin)]}
                            ]).to_string();
                            
                            if let Err(_) = ws_stream.send(ME::Text(subscribe_message)).await {
                                break;
                            }
                            println!("Subscribed to {}", new_coin);  // 디버그용
                        }
                        Some(Ok(message)) = ws_stream.next() => {
                            match message {
                                ME::Binary(bin_data) => {
                                    if let Ok(trade) = serde_json::from_str::<UpbitTrade>(
                                        &String::from_utf8_lossy(&bin_data)
                                    ) {
                                        println!("Received trade: {} {}", trade.code, trade.trade_price);  // 디버그용
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
impl Counter {
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(upbit_connection)
    }
    pub fn view(&self) -> Element<Message> {
        let coin_list = Column::new().spacing(2).width(Length::Fixed(200.0));

        let coin_list = self
            .coin_list
            .iter()
            .fold(coin_list, |column, (symbol, info)| {
                column.push(
                    button(
                        Container::new(
                            Row::new()
                                .spacing(10)
                                .push(Text::new(&*info.name))
                                .push(Text::new(info.change_percent.to_string())),
                            // .push(text(&change_text).size(14)),
                        )
                        // .style()
                        .width(Length::Fill)
                        .padding(10),
                    )
                    .padding(10)
                    .on_press(Message::SelectCoin(symbol.clone())),
                )
            });
        let canvas = Canvas::new(Chart::new(self.candlesticks.clone()))
            .width(Length::Fill)
            .height(Length::from(500));

        // 버튼 스타일 수정
        let add_button = button(text("Add Candlestick").size(8)).padding(10);
        let add_button2 = button(text("Add Candlestick").size(8)).padding(10);

        let remove_button = button(text("Remove Candlestick").size(8)).padding(10);

        Column::new()
            .push(
                Row::new()
                    .spacing(5)
                    // .push(styled_pick_list)
                    .push(add_button)
                    .push(remove_button),
            )
            .push(
                Row::new()
                    .push(coin_list)
                    .push(
                        container(canvas)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(20),
                    )
                    .push(
                        container(add_button2)
                            .width(300.)
                            .height(Length::Fill)
                            .padding(20),
                    ),
            )
            .into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::WebSocketInit(sender) => {
                self.ws_sender = Some(sender);
            }
            Message::SelectCoin(symbol) => {
                self.selected_coin = symbol.clone();
                // 새로운 코인의 캔들스틱 데이터 불러오기
                if let Ok(candles) = fetch_daily_candles(&format!("KRW-{}", symbol)) {
                    println!(
                        "가격 범위: {:?}",
                        candles.values().fold((f32::MAX, f32::MIN), |acc, c| {
                            (acc.0.min(c.low), acc.1.max(c.high))
                        })
                    );
                    self.candlesticks = candles;
                }

                // WebSocket에 새 코인 구독 요청
                if let Some(sender) = &self.ws_sender {
                    let _ = sender.clone().try_send(symbol);
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
            
                println!("Received trade for {}: price {}", trade_data.code, trade_data.trade_price);  // 디버그용
            
                let day_timestamp = timestamp - (timestamp % (24 * 60 * 60 * 1000));
                let trade_price = trade_data.trade_price as f32;

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
    
        // 캔들스틱 개수에 따라 너비 조정
        let candle_count = self.candlesticks.len() as f32;
        let available_width = bounds.width - 40.0;  // 여백 제외
        let fixed_candle_width = (available_width / candle_count).min(20.0);  // 최대 너비 제한
        let body_width = fixed_candle_width * 0.8;
    
        // 전체 차트 너비
        let total_width = fixed_candle_width * candle_count;
    
        // 스크롤 위치 계산
        let start_x = if state.auto_scroll {
            bounds.width - total_width - 20.0
        } else {
            20.0 + state.offset.min(0.0)  // 왼쪽으로만 스크롤 되도록
        };
    
        // 가격 범위 계산
        let (min_price, max_price) = self.candlesticks.values().fold((f32::MAX, f32::MIN), |acc, c| {
            (acc.0.min(c.low), acc.1.max(c.high))
        });
    
        // 가격대별 마진 설정
        let margin_percent = match max_price {
            p if p >= 10_000_000.0 => 0.01,  // BTC
            p if p >= 1_000_000.0 => 0.02,   // ETH
            p if p >= 100_000.0 => 0.03,     // SOL
            p if p >= 10_000.0 => 0.05,      // DOT
            _ => 0.1,                        // XRP
        };
    
        let margin = (max_price - min_price) * margin_percent;
        let min_price = min_price - margin;
        let max_price = max_price + margin;
        let price_diff = max_price - min_price;
        
        // Y축 스케일 계산
        let y_scale = (bounds.height - 40.0) / price_diff;
    
        // 나머지 그리기 코드는 동일...
    
        // 캔들스틱 그리기
        for (i, (_, candlestick)) in self.candlesticks.iter().enumerate() {
            let x = start_x + (i as f32 * fixed_candle_width);
    
            // 화면 밖의 캔들은 건너뛰기
            if x < -fixed_candle_width || x > bounds.width {
                continue;
            }
    
            let scale_price = |price: f32| -> f32 { 
                bounds.height - 20.0 - ((price - min_price) * y_scale)
            };
    

            let open_y = scale_price(candlestick.open);
            let close_y = scale_price(candlestick.close);
            let high_y = scale_price(candlestick.high);
            let low_y = scale_price(candlestick.low);

            let color = if candlestick.close >= candlestick.open {
                Color::from_rgb(0.8, 0.0, 0.0) // 상승 - 빨간색
            } else {
                Color::from_rgb(0.0, 0.0, 0.8) // 하락 - 파란색
            };

            // 심지
            frame.stroke(
                &canvas::Path::new(|builder| {
                    let center_x = x + (body_width / 2.0);
                    builder.move_to(Point::new(center_x, high_y));
                    builder.line_to(Point::new(center_x, low_y));
                }),
                canvas::Stroke::default().with_color(color).with_width(1.0),
            );

            // 몸통
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
    println!("markegt:{}", market);
    rt.block_on(async {
        let url = format!(
            "https://api.upbit.com/v1/candles/days?market={}&count=10000",
            market
        );
        let response = reqwest::get(&url).await?.json::<Vec<UpbitCandle>>().await?;

        Ok(response
            .into_iter()
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
            .collect())
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
    candle_acc_trade_price: f32,  // 누적 거래 금액
    candle_acc_trade_volume: f32, // 누적 거래량
    candle_date_time_kst: String, // 한국 표준시 날짜
    change_price: f32,            // 변동 가격
    change_rate: f32,             // 변동 비율
    high_price: f32,              // 고가
    low_price: f32,               // 저가
    opening_price: f32,           // 시가
    prev_closing_price: f32,      // 이전 종가
    timestamp: u64,               // 타임스탬프
    trade_price: f32,             // 현재가
}
