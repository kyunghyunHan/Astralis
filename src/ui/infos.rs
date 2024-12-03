use crate::Message;
use crate::Futurx;

use iced::{
    widget::{Column, Container, Row, Text},
    Color, Length,
};
pub fn coin_info(r: &Futurx) -> Column<'_, Message> {
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
pub fn account_info(r: &Futurx) -> Column<'static, Message> {
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

pub fn current_position(r: &Futurx) -> Container<'static, Message> {
    Container::new(
        Column::new()
            .spacing(10)
            .push(Text::new("Current Positions").size(16))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("USDT Balance:"))
                    .push(
                        Text::new(if let Some(info) = &r.account_info {
                            if let Some(asset) = info.assets.iter().find(|a| a.asset == "USDT") {
                                let available =
                                    asset.available_balance.parse::<f64>().unwrap_or(0.0);
                                format!("{:.2}", available)
                            } else {
                                "0.00".to_string()
                            }
                        } else {
                            "Loading...".to_string()
                        })
                        .size(16),
                    ),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new(format!("Position:")).size(16)) // 직접 format
                    .push(
                        Text::new(if let Some(info) = &r.account_info {
                            let symbol = format!("{}USDT", r.selected_coin);
                            if let Some(position) =
                                info.positions.iter().find(|p| p.symbol == symbol)
                            {
                                let amt = position.position_amt.parse::<f64>().unwrap_or(0.0);
                                let entry = position.entry_price.parse::<f64>().unwrap_or(0.0);
                                let pnl = position.unrealized_profit.parse::<f64>().unwrap_or(0.0);

                                if amt != 0.0 {
                                    let direction = if amt > 0.0 { "Long" } else { "Short" };
                                    format!(
                                        "{} {:.8} @ {:.2} (PNL: {:.2})",
                                        direction,
                                        amt.abs(),
                                        entry,
                                        pnl
                                    )
                                } else {
                                    "No Position".to_string()
                                }
                            } else {
                                "No Position".to_string()
                            }
                        } else {
                            "Loading...".to_string()
                        })
                        .size(16),
                    ),
            )
            .push(
                Row::new().push(Text::new("Size:").size(16)).push(
                    Text::new(if let Some(info) = r.coin_list.get(&r.selected_coin) {
                        let current_price = info.price; // 현재 마켓 가격

                        if let Some(position) = r.account_info.as_ref().and_then(|account| {
                            account
                                .positions
                                .iter()
                                .find(|p| p.symbol == format!("{}USDT", r.selected_coin))
                        }) {
                            let amt = position.position_amt.parse::<f64>().unwrap_or(0.0);
                            let size = amt * current_price; // Position Amount * Current Price
                            if size != 0.0 {
                                format!("{:.6} USDT", size)
                            } else {
                                "No Position".to_string()
                            }
                        } else {
                            "No Position".to_string()
                        }
                    } else {
                        "Loading...".to_string()
                    })
                    .size(16),
                ),
            )
            .push(
                Row::new().push(Text::new("Entry Price:").size(16)).push(
                    Text::new(if let Some(info) = &r.account_info {
                        let symbol = format!("{}USDT", r.selected_coin);
                        if let Some(position) = info.positions.iter().find(|p| p.symbol == symbol) {
                            let entry_price = position.entry_price.parse::<f64>().unwrap_or(0.0);
                            if entry_price > 0.0 {
                                format!("{:.6}", entry_price)
                            } else {
                                "No Position".to_string()
                            }
                        } else {
                            "No Position".to_string()
                        }
                    } else {
                        "Loading...".to_string()
                    })
                    .size(16),
                ),
            )
            .push(
                Row::new().push(Text::new(format!("ROE:")).size(16)).push(
                    Text::new(if let Some(info) = &r.account_info {
                        let symbol = format!("{}USDT", r.selected_coin);
                        if let Some(position) = info.positions.iter().find(|p| p.symbol == symbol) {
                            let initial_margin =
                                position.initial_margin.parse::<f64>().unwrap_or(1.0);
                            let unrealized_profit =
                                position.unrealized_profit.parse::<f64>().unwrap_or(0.0);

                            if initial_margin != 0.0 {
                                let roe = (unrealized_profit / initial_margin) * 100.0;
                                format!("{:.2}%", roe)
                            } else {
                                "0.00%".to_string()
                            }
                        } else {
                            "No Position".to_string()
                        }
                    } else {
                        "Loading...".to_string()
                    })
                    .size(16)
                    .color(if let Some(info) = &r.account_info {
                        if let Some(position) = info
                            .positions
                            .iter()
                            .find(|p| p.symbol == format!("{}USDT", r.selected_coin))
                        {
                            let roe = position.unrealized_profit.parse::<f64>().unwrap_or(0.0)
                                / position.initial_margin.parse::<f64>().unwrap_or(1.0)
                                * 100.0;
                            if roe >= 0.0 {
                                Color::from_rgb(0.0, 0.8, 0.0) // 이익일 때 초록색
                            } else {
                                Color::from_rgb(0.8, 0.0, 0.0) // 손실일 때 빨간색
                            }
                        } else {
                            Color::from_rgb(0.5, 0.5, 0.5) // 포지션 없을 때 회색
                        }
                    } else {
                        Color::from_rgb(0.5, 0.5, 0.5)
                    }),
                ),
            ),
    )
}
