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
        let (timezone_offset, market_open_hour) = match lang_type {
            LangType::Korean => (
                UtcOffset::from_hms(9, 0, 0).unwrap(), // UTC+9
                9,                                     // 한국 시장 시작 시간
            ),
            LangType::English => {
                // 3월~11월은 EDT (UTC-4), 나머지는 EST (UTC-5)
                let now = OffsetDateTime::now_utc();
                let month = now.month() as u8;

                match month {
                    3..=11 => (
                        UtcOffset::from_hms(-4, 0, 0).unwrap(), // EDT
                        9,                                      // 미국 시장 시작 시간 (9:30 AM ET)
                    ),
                    _ => (
                        UtcOffset::from_hms(-5, 0, 0).unwrap(), // EST
                        9,                                      // 미국 시장 시작 시간 (9:30 AM ET)
                    ),
                }
            }
        };

        println!("{}", timezone_offset);
        println!("{}", market_open_hour);

        let provider = yahoo::YahooConnector::new().unwrap();
        // let korea_offset = UtcOffset::from_hms(9, 0, 0).unwrap(); // UTC+9
        let end = OffsetDateTime::now_utc().to_offset(timezone_offset); // korea_offset 대신 timezone_offset 사용

        // 분봉과 일반 데이터 분리해서 처리
        let quotes = match stock_type {
            // 분봉 데이터
            "1m" | "2m" | "5m" | "15m" | "30m" | "60m" => {
                let start = match lang_type {
                    LangType::Korean => end
                        .replace_hour(market_open_hour)
                        .unwrap()
                        .replace_minute(0) // 한국 시장은 9:00 AM 시작
                        .unwrap()
                        .replace_second(0)
                        .unwrap()
                        .replace_nanosecond(0)
                        .unwrap(),
                    LangType::English => end
                        .replace_hour(market_open_hour)
                        .unwrap()
                        .replace_minute(30) // 미국 시장은 9:30 AM 시작
                        .unwrap()
                        .replace_second(0)
                        .unwrap()
                        .replace_nanosecond(0)
                        .unwrap(),
                };

                let resp = tokio_test::block_on(
                    provider.get_quote_history_interval(stock_name, start, end, stock_type),
                );

                // 에러 처리 추가
                match resp {
                    Ok(data) => match data.quotes() {
                        Ok(quotes) => quotes,
                        Err(e) => {
                            println!("Error parsing quotes: {:?}", e);
                            vec![] // 또는 적절한 에러 처리
                        }
                    },
                    Err(e) => {
                        println!("Error getting data: {:?}", e);
                        vec![] // 또는 적절한 에러 처리
                    }
                }
            }
            // 일봉, 주봉, 월봉 데이터
            _ => {
                let resp =
                    tokio_test::block_on(provider.get_quote_range(stock_name, stock_type, "30y"))
                        .unwrap();
                resp.quotes().unwrap()
            }
        };

        // println!("Data points: {}", quotes.len());

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
