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
pub fn coin_info(r: &RTarde) -> Column<'_, Message> {
    let coin_info = if let Some(info) = r.coin_list.get(&r.selected_coin) {
        Column::new()
            .spacing(10)
            .push(
                Container::new(
                    Column::new()
                        .push(Text::new(&info.name).size(28).width(Length::Fill))
                        .push(
                            Text::new(&info.symbol)
                                .size(14)
                                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                        ),
                )
                .padding(10)
                .width(Length::Fill),
            )
            .push(
                Container::new(Text::new(format!("{:.6} USDT", info.price)).size(32))
                    .padding(15)
                    .width(Length::Fill),
            )
    } else {
        Column::new().push(Text::new("Loading..."))
    };
    coin_info
}
pub fn account_info(r: &RTarde) -> Column<'_, Message> {
    Column::new()
        .spacing(10)
        .push(Text::new("Account Info").size(24))
        .push(
            Row::new()
                .spacing(10)
                .push(Text::new("Total Balance:"))
                .push(
                    Text::new(if let Some(info) = &r.account_info {
                        if let Some(asset) = info.assets.iter().find(|a| a.asset == "USDT") {
                            let balance = asset.wallet_balance.parse::<f64>().unwrap_or(0.0);
                            let pnl = asset.unrealized_profit.parse::<f64>().unwrap_or(0.0);
                            format!("{:.2} USDT (PNL: {:.2})", balance, pnl)
                        } else {
                            "0.00 USDT".to_string()
                        }
                    } else {
                        "Loading...".to_string()
                    })
                    .size(16),
                ),
        )
}
