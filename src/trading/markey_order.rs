use crate::execute_trade;
use crate::trading::TradeType;
use crate::uc;
use crate::AlertType;
use crate::Futurx;

/*
시장가 매수주문 처리 함수
- 숏포지션이 있으면 :전체 숏 포지션 청산
- 포지션이 없으면 : 설정된 주문 금액으로 새로운 롱 포지션 생성
- 실제 거래 실행 및 알림 메세지 생성

*/
pub fn market_buy(r: &mut Futurx) {
    //선택한 코인의 정보가 있는지 확인
    if let Some(info) = r.coin_list.get(&r.selected_coin) {
        //계정 정보가 있는지 확인
        if let Some(account_info) = &r.account_info {
            let symbol = format!("{}USDT", r.selected_coin);
            let price = info.price;

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
                    uc::MARKET_BUY_ORDER_PRICE / price
                }
            } else {
                // 포지션이 없다면 새로운 롱 포지션
                uc::MARKET_BUY_ORDER_PRICE / price
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

/*
시장가 매도 주문 처리 함수
- 롱 포지션이 있으면 전체 롱 포지션 청산
- 포지션이 없으면 설정된 주문 금액으로 새로운 숏 포지션 생성

*/
pub fn market_sell(r: &mut Futurx) {
    if let Some(info) = r.coin_list.get(&r.selected_coin) {
        if let Some(account_info) = &r.account_info {
            let symbol = format!("{}USDT", r.selected_coin);
            let price = info.price;

            let total_quantity = if let Some(position) =
                account_info.positions.iter().find(|p| p.symbol == symbol)
            {
                let current_position = position.position_amt.parse::<f64>().unwrap_or(0.0);
                if current_position > 0.0 {
                    // 롱 포지션이 있다면 전부 청산
                    current_position
                } else {
                    // 롱 포지션이 없다면 새로운 숏 포지션
                    uc::MARKET_SELL_ORDER_PRICE / price
                }
            } else {
                // 포지션이 없다면 새로운 숏 포지션
                uc::MARKET_SELL_ORDER_PRICE / price
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
