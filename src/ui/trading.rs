use crate::Message;
use crate::RTarde;

use iced::{
    widget::{
        button,
        canvas::{
            event::{self, Event},
            Canvas, Program,
        },
        checkbox, container, pick_list, Column, Container, Row, Space, Text,
    },
    Color, Element, Length, Pixels, Point, Rectangle, Size, Subscription, Theme,
};
pub fn order_buttons(r: &RTarde) -> Column<'static, Message> {
    Column::new()
        .spacing(10)
        .push(Text::new("Order").size(16))
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
        )
}
pub fn auto_trading_toggle(r: &RTarde) -> Container<'static, Message> {
    Container::new(
        Row::new()
            .spacing(10)
            .push(
                checkbox("Auto trading", r.auto_trading_enabled)
                    .on_toggle(|_| Message::ToggleAutoTrading),
            )
            .push(
                Text::new(if r.auto_trading_enabled {
                    "Auto trading on"
                } else {
                    "Auto trading off"
                })
                .size(14)
                .color(if r.auto_trading_enabled {
                    Color::from_rgb(0.0, 0.8, 0.0)
                } else {
                    Color::from_rgb(0.5, 0.5, 0.5)
                }),
            ),
    )
    .padding(10)
    .width(Length::Fill)
}
