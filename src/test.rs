use iced::{
    widget::{
        button, canvas,
        canvas::{Canvas, Program},
        column, text, Column, Container,
    },
    window::Settings,
    Color, Element, Length,
};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Increment,
    Decrement,
}

#[derive(Default)]
struct Counter {
    value: i32,
    prices: Vec<f32>, // 주식 가격 데이터
}

impl Counter {
    pub fn view(&self) -> Element<Message> {
        let canvas = Canvas::new(Chart {
            prices: self.prices.clone(),
        })
        .width(Length::Fill)
        .height(Length::Fill);

        // Container::new(canvas)
        //     .width(Length::from(110))
        //     .height(Length::from(110))
        //     .padding(20)
        //     .center_x(100)
        //     .center_y(100)
        //     .into()
        Column::new()
            .push(button("+").on_press(Message::Increment))
            .push(text(self.value).size(50))
            .push(button("-").on_press(Message::Decrement))
            .push(
                Container::new(canvas)
                    .width(Length::Fill)
                    // .height(Length::Units(300)),
            )
            .into()
        // Column::new()
        //     .push(button("+").on_press(Message::Increment))
        //     .push(text(self.value).size(50))
        //     .push(button("-").on_press(Message::Decrement))
        //     .push(
        //         Canvas::new(Chart {
        //             prices: self.prices.clone(),
        //         })
        //         .width(iced::Length::Fill)
        //         .height(iced::Length::Fill),
        //     )
        //     .into()
        // column![
        //     button("+").on_press(Message::Increment),
        //     text(self.value).size(50),
        //     button("-").on_press(Message::Decrement),
        //     // 차트 표시
        //     Canvas::new(Chart {
        //         prices: self.prices.clone()
        //     })
        //     .width(iced::Length::Fill)
        //     .height(iced::Length::Fill),
        // ]
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
                self.prices.push(self.value as f32); // 현재 값을 차트 데이터에 추가
            }
            Message::Decrement => {
                self.value -= 1;
                self.prices.push(self.value as f32); // 현재 값을 차트 데이터에 추가
            }
        }
    }
}

struct Chart {
    prices: Vec<f32>,
}

impl<Message> Program<Message> for Chart {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let path = canvas::Path::new(|builder| {
            for (i, price) in self.prices.iter().enumerate() {
                let x = i as f32 * 10.0; // x 좌표
                let y = bounds.height - price; // y 좌표 (높이에서 가격을 빼서 아래로 그리기)
                let point: iced::Point = (x, y).into();
                if i == 0 {
                    builder.move_to(point);
                } else {
                    builder.line_to(point);
                }
            }
        });

        frame.stroke(
            &path,
            canvas::Stroke {
                style: canvas::Style::Solid(Color::from_rgb(255.0, 0.0, 0.0)),
                width: 2.0,
                line_cap: canvas::LineCap::Butt,
                line_join: canvas::LineJoin::Miter,
                line_dash: canvas::LineDash::default(),
            },
        );

        vec![frame.into_geometry()]
    }
}

fn main() -> iced::Result {
    iced::run("A simple chart", Counter::update, Counter::view)
}
