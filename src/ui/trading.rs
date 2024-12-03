use crate::utils::constant as uc;
use crate::Message;
use crate::RTarde;
use iced::widget::button::Status;
use iced::widget::button::Style;
use iced::Background;
use iced::Border;
use iced::Shadow;
use iced::{
    widget::{button, checkbox, Column, Container, Row, Text},
    Color, Length,
};
const BUTTON_ROUND: f32 = 10.;
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
                        .style(|_, status| {
                            if status != Status::Hovered {
                                Style {
                                    background: Some(Background::Color(Color::from_rgb(
                                        0.1, 0.3, 0.7,
                                    ))),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            } else {
                                Style {
                                    background: Some(Background::Color(Color::from_rgb(
                                        0.3, 0.5, 0.8,
                                    ))),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            }
                        })
                        .on_press(Message::MarketBuy),
                )
                .push(
                    button(Text::new("Short Market"))
                        .width(Length::Fill)
                        .style(|_, status| {
                            if status != Status::Hovered {
                                // 기본 상태: 어두운 빨간색
                                Style {
                                    background: Some(Background::Color(uc::DAKR_RED)),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            } else {
                                // 호버 상태: 밝은 빨간색
                                Style {
                                    background: Some(Background::Color(uc::BRIGHT_RED)),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            }
                        })
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
