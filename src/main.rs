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
        column, container, pick_list, text, Column, Container, PickList,
    },
    Color, Element, Length, Point, Rectangle, Size, Subscription,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
enum State {
    Disconnected,
    Connected(WebSocketStream<MaybeTlsStream<TcpStream>>),
}
#[derive(Debug, Clone)]
pub enum Message {
    AddCandlestick,
    RemoveCandlestick,
    FruitSelected(Fruit),
    Error,
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
    candlesticks: BTreeMap<u64, Candlestick>, // Vec에서 BTreeMap으로 변경
    selected_option: Option<Fruit>,
    auto_scroll: bool,
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
    candlesticks: BTreeMap<u64, Candlestick>, // Vec 에서 BTreeMap으로 변경
    state: ChartState,
}

impl Chart {
    fn new(candlesticks: BTreeMap<u64, Candlestick>) -> Self {
        Self {
            candlesticks,
            state: ChartState {
                auto_scroll: true,
                ..ChartState::default()
            },
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
        let candlesticks = fetch_daily_candles().unwrap_or_else(|_| {
            let mut default_map = BTreeMap::new();
            default_map.insert(
                0,
                Candlestick {
                    open: 100.0,
                    close: 110.0,
                    high: 115.0,
                    low: 95.0,
                },
            );
            default_map
        });

        Self {
            candlesticks, // BTreeMap 직접 사용
            timer_enabled: true,
            selected_option: None,
            auto_scroll: true,
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
enum Input {
    Subscribe,
}

fn upbit_connection() -> impl Stream<Item = Message> {
    async_stream::stream! {
        let url = Url::parse("wss://api.upbit.com/websocket/v1").unwrap();

        loop {
            if let Ok((ws_stream, _)) = connect_async(url.clone()).await {
                let (mut write, mut read) = ws_stream.split();

                let subscribe_msg = json!([{
                    "ticket": "UNIQUE_TICKET",
                    "type": "trade",
                    "codes": ["KRW-BTC"],
                }]);

                if write.send(subscribe_msg.to_string().into()).await.is_ok() {
                    while let Some(Ok(msg)) = read.next().await {
                        if let Ok(_data) = serde_json::from_slice::<UpbitWebSocketData>(&msg.into_data()) {
                            yield Message::AddCandlestick;
                            println!("{:?}",_data);
                        }
                    }
                }
            }
            yield Message::Error;
            // 연결이 끊기면 잠시 대기 후 재시도
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
impl Counter {
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(upbit_connection)
    }
    pub fn view(&self) -> Element<Message> {
        let canvas = Canvas::new(Chart::new(self.candlesticks.clone()))
            .width(Length::Fill)
            .height(Length::from(500));

        let fruits = [
            Fruit::Apple,
            Fruit::Orange,
            Fruit::Strawberry,
            Fruit::Tomato,
        ];

        // 버튼 스타일 수정
        let add_button = button(text("Add Candlestick").size(16)).padding(10);

        let remove_button = button(text("Remove Candlestick").size(16)).padding(10);

        let styled_pick_list =
            pick_list(fruits, self.selected_option, Message::FruitSelected).padding(10);

        Column::new()
            .push(container(text("Candlestick Chart").size(24)).padding(20))
            .push(
                Row::new()
                    .spacing(10)
                    .push(styled_pick_list)
                    .push(add_button)
                    .push(remove_button),
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
            Message::AddCandlestick => {
                let last_close = self
                    .candlesticks
                    .values()
                    .last()
                    .map(|c| c.close)
                    .unwrap_or(100.0);

                let new_timestamp = self.candlesticks.keys().last().map(|k| k + 1).unwrap_or(0);

                self.candlesticks.insert(
                    new_timestamp,
                    Candlestick {
                        open: last_close,
                        close: last_close,
                        high: last_close,
                        low: last_close,
                    },
                );
                self.auto_scroll = true;
            }
            Message::RemoveCandlestick => {
                if let Some(&last_key) = self.candlesticks.keys().last() {
                    self.candlesticks.remove(&last_key);
                }
                self.auto_scroll = true;
            }
            Message::FruitSelected(fruit) => {
                self.selected_option = Some(fruit);
            }
            Message::Error => {
                // WebSocket 에러 처리
                println!("WebSocket connection error");
                // 필요한 경우 재연결 로직 추가
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

        // 차트의 스케일 계산
        let price_range = self
            .candlesticks
            .values() // 값만 가져오기
            .fold((f32::MAX, f32::MIN), |acc, c| {
                (acc.0.min(c.low), acc.1.max(c.high))
            });

        let price_diff = price_range.1 - price_range.0;
        let y_scale = (bounds.height - 40.0) / price_diff;

        let fixed_candle_width = 20.0;
        let body_width = fixed_candle_width * 0.8;

        let total_width = fixed_candle_width * self.candlesticks.len() as f32;

        let start_x = if state.auto_scroll && total_width > bounds.width {
            bounds.width - total_width - 20.0
        } else {
            20.0 + state.offset
        };

        // BTreeMap의 값들을 순회
        for (i, (_, candlestick)) in self.candlesticks.iter().enumerate() {
            let x = start_x + (i as f32 * fixed_candle_width);

            if x < -fixed_candle_width || x > bounds.width {
                continue;
            }

            let scale_price =
                |price: f32| -> f32 { bounds.height - 20.0 - ((price - price_range.0) * y_scale) };
            let open_y = scale_price(candlestick.open);
            let close_y = scale_price(candlestick.close);
            let high_y = scale_price(candlestick.high);
            let low_y = scale_price(candlestick.low);
            let color = if candlestick.close >= candlestick.open {
                Color::from_rgb(0.0, 0.8, 0.0)
            } else {
                Color::from_rgb(0.8, 0.0, 0.0)
            };

            // 심지 그리기
            frame.stroke(
                &canvas::Path::new(|builder| {
                    let center_x = x + (body_width / 2.0);
                    builder.move_to(Point::new(center_x, high_y));
                    builder.line_to(Point::new(center_x, low_y));
                }),
                canvas::Stroke::default().with_color(color).with_width(1.0),
            );

            // 몸통 그리기
            let body_height = (close_y - open_y).abs();
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
fn fetch_daily_candles() -> Result<BTreeMap<u64, Candlestick>, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        let url = "https://api.upbit.com/v1/candles/days?market=KRW-BTC&count=200";
        let response = reqwest::get(url).await?.json::<Vec<UpbitCandle>>().await?;

        // UpbitCandle을 우리의 Candlestick 형식으로 변환
        Ok(response
            .into_iter()
            .map(|candle| {
                (
                    candle.timestamp, // BTreeMap의 키로 timestamp 사용
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
        .run()
}

#[derive(Debug, Deserialize)]
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
