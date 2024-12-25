use crate::utils::constant as uc;
use crate::Futurx;
use crate::Message;
use iced::widget::button::Status;
use iced::widget::button::Style;
use iced::Background;
use iced::Border;
use iced::Shadow;
use iced::{
    widget::{button, checkbox, Column, Container, Row, Text},
    Color, Length,
};
/*  주문버튼
   - 빨간색 : 롱 포지션
   - 파란색 : 숏 포지션
*/
pub fn order_buttons(r: &Futurx) -> Column<'static, Message> {
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
                                    background: Some(Background::Color(uc::BRIGHT_BLUE)),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(uc::BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            } else {
                                Style {
                                    background: Some(Background::Color(uc::DAKR_BLUE)),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(uc::BUTTON_ROUND),
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
                                    border: Border::default().rounded(uc::BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            } else {
                                // 호버 상태: 밝은 빨간색
                                Style {
                                    background: Some(Background::Color(uc::BRIGHT_RED)),
                                    text_color: Color::WHITE,
                                    border: Border::default().rounded(uc::BUTTON_ROUND),
                                    shadow: Shadow::default(),
                                }
                            }
                        })
                        .on_press(Message::MarketSell),
                ),
        )
}

/*
자동 매매 버튼
*/
pub fn auto_trading_toggle(r: &Futurx) -> Container<'static, Message> {
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
                    uc::BRIGH_GREEN
                } else {
                    uc::BRIGH_GRAY
                }),
            ),
    )
    .padding(10)
    .width(Length::Fill)
}
