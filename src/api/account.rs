use crate::api::FuturesAccountInfo;
use crate::utils::hmac_sha256;
use crate::Message;
use crate::Trade;
use async_stream::stream;
use futures_util::Stream; // Add this at the top with other imports

use std::env;
pub fn binance_account_connection() -> impl Stream<Item = Message> {
    stream! {

        let api_key = match env::var("BINANCE_API_KEY") {
            Ok(key) => {
                println!("API KEY found");
                key
            }
            Err(e) => {
                println!("API KEY error: {}", e);
                yield Message::FetchError("API KEY not found".to_string());
                return;
            }
        };

        let api_secret = match env::var("BINANCE_API_SECRET") {
            Ok(secret) => {
                println!("API SECRET found");
                secret
            }
            Err(e) => {
                println!("API SECRET error: {}", e);
                yield Message::FetchError("API SECRET not found".to_string());
                return;
            }
        };

        let client = reqwest::Client::new();

        loop {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let query = format!("timestamp={}", timestamp);
            let signature = hmac_sha256(&api_secret, &query);

            // 퓨처스 계정 정보 엔드포인트로 변경
            let url = format!(
                "https://fapi.binance.com/fapi/v2/account?{}&signature={}",
                query, signature
            );

            // println!("Requesting futures account info: {}", url);

            match client
                .get(&url)
                .header("X-MBX-APIKEY", &api_key)
                .send()
                .await
            {
                Ok(response) => {
                    // println!("Response status: {}", response.status());

                    if response.status().is_success() {
                        let text = response.text().await.unwrap_or_default();
                        // println!("Raw response: {}", text);

                        match serde_json::from_str::<FuturesAccountInfo>(&text) {
                            Ok(account_info) => {
                                // 계정 정보 업데이트
                                for position in &account_info.positions {
                                    println!("this:{:?}",position);
                                    if position.position_amt.parse::<f64>().unwrap_or(0.0) != 0.0 {
                                        let trades_query = format!(
                                            "symbol={}&limit=100&timestamp={}",
                                            position.symbol,
                                            timestamp
                                        );
                                        let trades_signature = hmac_sha256(&api_secret, &trades_query);
                                        let trades_url = format!(
                                            "https://fapi.binance.com/fapi/v1/userTrades?{}&signature={}",
                                            trades_query, trades_signature
                                        );

                                        if let Ok(trades_response) = client
                                            .get(&trades_url)
                                            .header("X-MBX-APIKEY", &api_key)
                                            .send()
                                            .await
                                        {
                                            if let Ok(trades) = trades_response.json::<Vec<Trade>>().await {
                                                let mut total_buy_amount = 0.0;
                                                let mut total_buy_quantity = 0.0;

                                                for trade in trades {
                                                    if trade.is_buyer {
                                                        let price = trade.price.parse::<f64>().unwrap_or(0.0);
                                                        let quantity = trade.qty.parse::<f64>().unwrap_or(0.0);
                                                        total_buy_amount += price * quantity;
                                                        total_buy_quantity += quantity;
                                                    }
                                                }

                                                if total_buy_quantity > 0.0 {
                                                    let avg_price = total_buy_amount / total_buy_quantity;
                                                    let symbol = position.symbol.replace("USDT", "");
                                                    yield Message::UpdateAveragePrice(symbol, avg_price);
                                                }
                                            }
                                        }
                                    }
                                }
                                yield Message::UpdateAccountInfo(account_info);
                            }
                            Err(e) => {
                                println!("Failed to parse futures account info: {} \nResponse: {}", e, text);
                                yield Message::FetchError(format!("Parse error: {}", e));
                            }
                        }
                    } else {
                        let error = response.text().await.unwrap_or_default();
                        println!("API error response: {}", error);
                        yield Message::FetchError(format!("API error: {}", error));
                    }
                }
                Err(e) => {
                    println!("Request error: {}", e);
                    yield Message::FetchError(format!("Request failed: {}", e));
                }
            }

            // println!("Sleeping for 5 seconds...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}
