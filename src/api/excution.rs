use crate::adjust_precision;
use crate::get_symbol_info;
use crate::utils::hmac_sha256;
use crate::AlertType;
use crate::TradeType;
use futures_util::SinkExt;
use iced::futures::{channel::mpsc, StreamExt};

use std::env;
pub async fn execute_trade(
    selected_coin: String,
    trade_type: TradeType,
    price: f64,
    mut amount: f64,
    mut alert_sender: mpsc::Sender<(String, AlertType)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let symbol = format!("{}USDT", selected_coin);

    // 심볼 정보 가져오기
    let (quantity_precision, _) = get_symbol_info(&symbol).await?;

    // 수량 정밀도 조정
    amount = adjust_precision(amount, quantity_precision);

    let api_key = env::var("BINANCE_API_KEY")?;
    let api_secret = env::var("BINANCE_API_SECRET")?;
    let timestamp = chrono::Utc::now().timestamp_millis();

    let side = match trade_type {
        TradeType::Buy => "BUY",
        TradeType::Sell => "SELL",
    };

    let params = format!(
        "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}",
        symbol, side, amount, timestamp
    );

    println!("Sending order with params: {}", params);

    let signature = hmac_sha256(&api_secret, &params);
    let url = format!(
        "https://fapi.binance.com/fapi/v1/order?{}&signature={}",
        params, signature
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-MBX-APIKEY", &api_key)
        .send()
        .await?;
    println!("Response Status: {:?}", response.status());
    println!("Response Headers: {:?}", response.headers());
    let status = response.status();
    println!("Response status: {}", status);

    // 응답 텍스트 얻기
    let response_text = response.text().await?;
    println!("Raw Response Body: {}", response_text);
    match serde_json::from_str::<serde_json::Value>(&response_text) {
        Ok(json) => {
            println!("Parsed JSON Response: {:#?}", json);
            // JSON의 각 필드 확인
            println!("Fields:");
            if let Some(order_id) = json.get("orderId") {
                println!("- orderId: {:?}", order_id);
            }
            if let Some(executed_qty) = json.get("executedQty") {
                println!("- executedQty: {:?}", executed_qty);
            }
            if let Some(avg_price) = json.get("avgPrice") {
                println!("- avgPrice: {:?}", avg_price);
            }
        }
        Err(e) => println!("Failed to parse JSON: {}", e),
    };
    if status.is_success() {
        let result: serde_json::Value = serde_json::from_str(&response_text)?;

        let executed_qty = result["executedQty"].as_str().unwrap_or("0");
        let avg_price = result["avgPrice"].as_str().unwrap_or("0");
        let order_id = result["orderId"].as_u64().unwrap_or(0);

        let message = format!(
            "{} order success (order number: {}):\nquantity: {} {}\nAverage price: {} USDT\nTotal amount: {:.2} USDT",
            match trade_type {
                TradeType::Buy => "롱",
                TradeType::Sell => "숏",
            },
            order_id,
            executed_qty,
            selected_coin,
            avg_price,
            executed_qty.parse::<f64>().unwrap_or(0.0) * avg_price.parse::<f64>().unwrap_or(0.0)
        );

        println!("Order success: {}", message);

        alert_sender
            .send((
                message,
                match trade_type {
                    TradeType::Buy => AlertType::Buy,
                    TradeType::Sell => AlertType::Sell,
                },
            ))
            .await?;

        Ok(())
    } else {
        let error_message = format!("order failed: {}", response_text);
        println!("Order failed: {}", error_message);

        alert_sender
            .send((error_message.clone(), AlertType::Error))
            .await?;

        Err(error_message.into())
    }
}
