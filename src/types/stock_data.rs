use crate::types::StockData;
use rayon::prelude::*;
use std::collections::BTreeMap;
use time::Duration;
use time::Weekday;
use time::{OffsetDateTime, UtcOffset}; // 하나만 유지
use tokio_test;
use yahoo_finance_api as yahoo;

use super::LangType; // time 크레이트의 Weekday를 사용합니다.
impl StockData {
    pub fn get_data(
        stock_name: &str,
        stock_type: &str,
        lang_type: &LangType,
    ) -> BTreeMap<u64, StockData> {
        println!("Type: {}", stock_type);

        let provider = yahoo::YahooConnector::new().unwrap();
        let korea_offset = UtcOffset::from_hms(9, 0, 0).unwrap(); // UTC+9
        let end = OffsetDateTime::now_utc().to_offset(korea_offset);

        let start = match end.weekday() {
            Weekday::Monday => end - Duration::days(3), // 월요일인 경우 금요일부터 (주말 건너뛰기)
            Weekday::Sunday => end - Duration::days(2), // 일요일인 경우 금요일부터
            Weekday::Saturday => end - Duration::days(1), // 토요일인 경우 금요일부터
            _ => end - Duration::days(1),               // 화~금요일은 전일부터
        }
        .replace_hour(9)
        .unwrap()
        .replace_minute(0)
        .unwrap()
        .replace_second(0)
        .unwrap()
        .replace_nanosecond(0)
        .unwrap();
        let lang = match lang_type {
            LangType::English => {}
            _ => {}
        };
        println!("{}", end.weekday());
        let start = match end.weekday() {
            Weekday::Monday => end - Duration::days(3), // 월요일인 경우 금요일부터 (주말 건너뛰기)
            Weekday::Sunday => end - Duration::days(2), // 일요일인 경우 금요일부터
            Weekday::Saturday => end - Duration::days(1), // 토요일인 경우 금요일부터
            _ => end,                                   // 화~금요일은 전일부터
        }
        .replace_hour(9)
        .unwrap()
        .replace_minute(0)
        .unwrap()
        .replace_second(0)
        .unwrap()
        .replace_nanosecond(0)
        .unwrap();
        println!("{}", start);
        println!("{}", end);
        // 분봉과 일반 데이터 분리해서 처리
        let quotes = match stock_type {
            // 분봉 데이터
            "1m" | "2m" | "5m" | "15m" | "30m" | "60m" => {
                let resp = tokio_test::block_on(
                    provider.get_quote_history_interval(stock_name, start, end, stock_type),
                )
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
