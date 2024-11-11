use crate::types::StockData;
use rayon::prelude::*;
use std::collections::BTreeMap;
use time::Duration;
use time::OffsetDateTime; // 하나만 유지
use time::Weekday;
use tokio_test;
use yahoo_finance_api as yahoo; // time 크레이트의 Weekday를 사용합니다.

use yahoo_finance_api::time::macros::datetime;
impl StockData {
    // pub fn get_data(stock_name: &str, stock_type: &str) -> BTreeMap<u64, StockData> {
    //     let provider = yahoo::YahooConnector::new().unwrap();

    //     let end = OffsetDateTime::now_utc();
    //     println!("{}",end.weekday());
    //     let start = match end.weekday() {
    //         Weekday::Monday => end - Duration::days(3),
    //         Weekday::Tuesday => end - Duration::days(1),
    //         Weekday::Wednesday => end - Duration::days(1),
    //         Weekday::Thursday => end - Duration::days(1),
    //         Weekday::Friday => end - Duration::days(1),
    //         _ => end - Duration::days(1), // 주말인 경우에는 1일 전부터 데이터를 요청합니다.
    //     };
    //     let interval = "1m"; // 1분봉

    //     // 분봉 데이터를 요청할 수 있는 메서드 호출 (라이브러리에 따라 메서드가 다를 수 있음)
    //     let resp = tokio_test::block_on(provider.get_quote_history_interval(stock_name, start, end, interval))
    //         .unwrap();
    //     let quotes = resp.quotes().unwrap();

    //     println!("{}", quotes.len());

    //     BTreeMap::default()
    // }

    pub fn get_data(stock_name: &str, stock_type: &str) -> BTreeMap<u64, StockData> {
        println!("Type: {}", stock_type);

        let provider = yahoo::YahooConnector::new().unwrap();
        let end = OffsetDateTime::now_utc();
        let start = match end.weekday() {
            Weekday::Monday => end - Duration::days(1),
            Weekday::Tuesday => end - Duration::days(1),
            Weekday::Wednesday => end - Duration::days(1),
            Weekday::Thursday => end - Duration::days(1),
            Weekday::Friday => end - Duration::days(1),
            _ => end - Duration::days(1), // 주말인 경우에는 1일 전부터 데이터를 요청합니다.
        };

        // 분봉과 일반 데이터 분리해서 처리
        let quotes = match stock_type {
            // 분봉 데이터
            "1m" | "2m" | "5m" | "15m" | "30m" | "60m" => {
                let resp = tokio_test::block_on(provider.get_quote_history_interval(stock_name, start, end,stock_type))
                    .unwrap();
                resp.quotes().unwrap()
            }
            // 일봉, 주봉, 월봉 데이터
            _ => {
                let resp =
                    tokio_test::block_on(provider.get_quote_range(stock_name, stock_type, "30y"))
                        .unwrap();
                resp.quotes().unwrap()
            }
        };

        println!("Data points: {}", quotes.len());

        // 정규화 계수
        let normalize_factor = match stock_type {
            "1m" => 60,          // 1분
            "2m" => 60 * 2,      // 2분
            "5m" => 60 * 5,      // 5분
            "15m" => 60 * 15,    // 15분
            "30m" => 60 * 30,    // 30분
            "60m" => 60 * 60,    // 1시간
            "1mo" => 86400 * 30, // 월 단위
            "1wk" => 86400 * 7,  // 주 단위
            "1d" => 86400,       // 일 단위
            _ => 86400,          // 기본값
        };

        let timestamps: Vec<u64> = quotes
            .iter()
            .map(|quote| quote.timestamp as u64 / normalize_factor)
            .collect();

        if let (Some(&first), Some(&last)) = (timestamps.first(), timestamps.last()) {
            println!(
                "First normalized time: {}, Last normalized time: {}",
                first, last
            );
            println!("Total time periods: {}", last - first + 1);
        }

        quotes
            .into_par_iter()
            .map(|quote| {
                let candle = StockData {
                    open: quote.open as f64,
                    high: quote.high as f64,
                    low: quote.low as f64,
                    close: quote.close as f64,
                    volume: quote.volume as f64,
                };
                let normalized_timestamp = quote.timestamp as u64 / normalize_factor;
                (normalized_timestamp, candle)
            })
            .collect()
    }
}
