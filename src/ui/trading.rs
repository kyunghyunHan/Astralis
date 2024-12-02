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
pub fn order_buttons(r: &RTarde) -> Column<'_, Message> {
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
