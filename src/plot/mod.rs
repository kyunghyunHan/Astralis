// #[derive(Debug, Clone)]
// pub struct Candlestick {
//     pub open: f32,
//     pub close: f32,
//     pub high: f32,
//     pub low: f32,
// }
// use iced::{
//     mouse, time,
//     widget::{
//         button, canvas,
//         canvas::{
//             event::{self, Event},
//             Canvas, Program,
//         },
//         column, container, pick_list, text, Column, Container, PickList,
//     },
//     Color, Element, Length, Point, Rectangle, Size, Subscription,
// };
// use std::thread::sleep;
// use std::time::{Duration, Instant};
// pub struct Chart {
//     candlesticks: Vec<Candlestick>,
//     state: ChartState,
// }

// impl Chart {
//     pub fn new(candlesticks: Vec<Candlestick>) -> Self {
//         Self {
//             candlesticks,
//             state: ChartState::default(), // ChartState를 새로운 인스턴스로 초기화

//         }
//     }
// }

// // #[derive(Default)]
// pub struct ChartState {
//     offset: f32,
//     dragging: bool,
//     drag_start: Point,
//     last_offset: f32,
//     auto_scroll: bool,
//     scroll_offset: f32,
//     last_time: Instant, // 추가된 필드
// }

// // Default trait의 수동 구현
// impl Default for ChartState {
//     fn default() -> Self {
//         Self {
//             offset: 0.0,
//             dragging: false,
//             drag_start: Point::new(0.0, 0.0),
//             last_offset: 0.0,
//             auto_scroll: false,
//             scroll_offset: 0.0,
//             last_time: Instant::now(), // 현재 시간을 설정
//         }
//     }
// }
// impl<Message> Program<Message> for Chart {
//     type State = ChartState;
 
//     fn update(
//         &self,
//         state: &mut Self::State,
//         event: Event,
//         bounds: Rectangle,
//         cursor: mouse::Cursor,
//     ) -> (event::Status, Option<Message>) {
//         let cursor_position = if let Some(position) = cursor.position() {
//             position
//         } else {
//             return (event::Status::Ignored, None);
//         };
//         let now = Instant::now();

//         let cursor_position = if let Some(position) = cursor.position() {
//             position
//         } else {
//             return (event::Status::Ignored, None);
//         };

//         // match event {
//         //     Event::Mouse(mouse_event) => match mouse_event {
//         //         mouse::Event::ButtonPressed(mouse::Button::Left) => {
//         //             state.dragging = true;
//         //             state.drag_start = cursor_position;
//         //             state.last_offset = state.offset;
//         //             state.auto_scroll = false;
//         //             (event::Status::Captured, None)
//         //         }
//         //         mouse::Event::ButtonReleased(mouse::Button::Left) => {
//         //             state.dragging = false;
//         //             (event::Status::Captured, None)
//         //         }

//         //         mouse::Event::CursorMoved { .. } => {
//         //             if state.dragging {
//         //                 let delta_x = cursor_position.x - state.drag_start.x;
//         //                 state.offset = state.last_offset + delta_x;
//         //                 (event::Status::Captured, None)
//         //             } else {
//         //                 (event::Status::Ignored, None)
//         //             }
//         //         }

//         //         _ => (event::Status::Ignored, None),
//         //     },

//         //     _ => (event::Status::Ignored, None),
//         // }
//         (event::Status::Ignored, None)
//     }

//     fn draw(
//         &self,
//         state: &Self::State,
//         renderer: &iced::Renderer,
//         _theme: &iced::Theme,
//         bounds: Rectangle,
//         _cursor: mouse::Cursor,
//     ) -> Vec<canvas::Geometry> {
//         let mut frame = canvas::Frame::new(renderer, bounds.size());
//         if self.candlesticks.is_empty() {
//             return vec![frame.into_geometry()];
//         }
//         println!("{}", "draw");
//         let price_range = self
//             .candlesticks
//             .iter()
//             .fold((f32::MAX, f32::MIN), |acc, c| {
//                 (acc.0.min(c.low), acc.1.max(c.high))
//             });

//         let price_diff = price_range.1 - price_range.0;
//         let y_scale = (bounds.height - 40.0) / price_diff;

//         let fixed_candle_width = 20.0;
//         let body_width = fixed_candle_width * 0.8;
//         let total_width = fixed_candle_width * self.candlesticks.len() as f32;

//         let scroll_amount = if state.auto_scroll {
//             state.scroll_offset
//         } else {
//             0.0
//         };
//         let start_x = if total_width > bounds.width {
//             20.0 - scroll_amount + state.offset
//         } else {
//             20.0 + state.offset
//         };
//         for (i, candlestick) in self.candlesticks.iter().enumerate() {
//             let x = start_x + (i as f32 * fixed_candle_width);

//             if x < -fixed_candle_width || x > bounds.width {
//                 continue;
//             }

//             let scale_price =
//                 |price: f32| -> f32 { bounds.height - 20.0 - ((price - price_range.0) * y_scale) };
//             let open_y = scale_price(candlestick.open);
//             let close_y = scale_price(candlestick.close);
//             let high_y = scale_price(candlestick.high);
//             let low_y = scale_price(candlestick.low);

//             let color = if candlestick.close >= candlestick.open {
//                 Color::from_rgb(0.0, 0.8, 0.0)
//             } else {
//                 Color::from_rgb(0.8, 0.0, 0.0)
//             };

//             frame.stroke(
//                 &canvas::Path::new(|builder| {
//                     let center_x = x + (body_width / 2.0);
//                     builder.move_to(Point::new(center_x, high_y));
//                     builder.line_to(Point::new(center_x, low_y));
//                 }),
//                 canvas::Stroke::default().with_color(color).with_width(1.0),
//             );

//             let body_height = (close_y - open_y).abs();
//             let body_y = close_y.min(open_y);
//             frame.fill_rectangle(
//                 Point::new(x, body_y),
//                 Size::new(body_width, body_height),
//                 color,
//             );
//             loop {
//                 println!("1");
//                 sleep(Duration::from_secs(3));
//                 return vec![frame.into_geometry()];
//             }
//         }

//         vec![frame.into_geometry()]
//     }
// }

pub mod plot;