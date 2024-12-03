use crate::execute_trade;
use crate::trading::TradeType;
use crate::AlertType;
use crate::RTarde;
pub fn market_buy(r: &mut RTarde) {
    if let Some(info) = r.coin_list.get(&r.selected_coin) {
        if let Some(account_info) = &r.account_info {
            let symbol = format!("{}USDT", r.selected_coin);
            let price = info.price;
            let fixed_usdt = 6.0;

            // 현재 포지션 확인
            let total_quantity = if let Some(position) =
                account_info.positions.iter().find(|p| p.symbol == symbol)
            {
                let current_position = position.position_amt.parse::<f64>().unwrap_or(0.0);
                if current_position < 0.0 {
                    // 숏 포지션이 있다면 전부 청산
                    current_position.abs()
                } else {
                    // 숏 포지션이 없다면 새로운 롱 포지션
                    fixed_usdt / price
                }
            } else {
                // 포지션이 없다면 새로운 롱 포지션
                fixed_usdt / price
            };

            if total_quantity > 0.0 {
                let selected_coin = r.selected_coin.clone();
                let alert_sender = r.alert_sender.clone();

                let runtime = tokio::runtime::Handle::current();
                runtime.spawn(async move {
                    if let Err(e) = execute_trade(
                        selected_coin.clone(),
                        TradeType::Buy,
                        price,
                        total_quantity,
                        alert_sender,
                    )
                    .await
                    {
                        println!("시장가 매수 실패: {:?}", e);
                    }
                });

                let message =
                    if account_info.positions.iter().any(|p| {
                        p.symbol == symbol && p.position_amt.parse::<f64>().unwrap_or(0.0) < 0.0
                    }) {
                        format!(
                        "Closing Short Position:\nQuantity: {:.8} {}\nEstimated Cost: {:.4} USDT",
                        total_quantity, r.selected_coin, total_quantity * price
                    )
                    } else {
                        format!(
                            "New Long Position:\nQuantity: {:.8} {}\nEstimated Cost: {:.4} USDT",
                            total_quantity,
                            r.selected_coin,
                            total_quantity * price
                        )
                    };

                r.add_alert(message, AlertType::Info);
            }
        } else {
            r.add_alert(
                "Account information cannot be registered.".to_string(),
                AlertType::Error,
            );
        }
    }
}
pub fn market_sell(r: &mut RTarde) {
    if let Some(info) = r.coin_list.get(&r.selected_coin) {
        if let Some(account_info) = &r.account_info {
            let symbol = format!("{}USDT", r.selected_coin);
            let price = info.price;
            let fixed_usdt = 6.0;

            // 현재 포지션 확인
            let total_quantity = if let Some(position) =
                account_info.positions.iter().find(|p| p.symbol == symbol)
            {
                let current_position = position.position_amt.parse::<f64>().unwrap_or(0.0);
                if current_position > 0.0 {
                    // 롱 포지션이 있다면 전부 청산
                    current_position
                } else {
                    // 롱 포지션이 없다면 새로운 숏 포지션
                    fixed_usdt / price
                }
            } else {
                // 포지션이 없다면 새로운 숏 포지션
                fixed_usdt / price
            };

            if total_quantity > 0.0 {
                let selected_coin = r.selected_coin.clone();
                let alert_sender = r.alert_sender.clone();

                let runtime = tokio::runtime::Handle::current();
                runtime.spawn(async move {
                    if let Err(e) = execute_trade(
                        selected_coin.clone(),
                        TradeType::Sell,
                        price,
                        total_quantity,
                        alert_sender,
                    )
                    .await
                    {
                        println!("시장가 매도 실패: {:?}", e);
                    }
                });

                let message =
                    if account_info.positions.iter().any(|p| {
                        p.symbol == symbol && p.position_amt.parse::<f64>().unwrap_or(0.0) > 0.0
                    }) {
                        format!(
                        "Closing Long Position:\nQuantity: {:.8} {}\nEstimated Cost: {:.4} USDT",
                        total_quantity, r.selected_coin, total_quantity * price
                    )
                    } else {
                        format!(
                            "New Short Position:\nQuantity: {:.8} {}\nEstimated Cost: {:.4} USDT",
                            total_quantity,
                            r.selected_coin,
                            total_quantity * price
                        )
                    };

                r.add_alert(message, AlertType::Info);
            }
        } else {
            r.add_alert(
                "Account information cannot be registered.".to_string(),
                AlertType::Error,
            );
        }
    }
}
