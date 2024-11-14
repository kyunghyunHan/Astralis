use crate::types::StockData;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::Mutex;
use time::Duration;
use time::{OffsetDateTime, UtcOffset};
use tokio_test;
use yahoo_finance_api as yahoo;

use super::LangType;

impl StockData {
    const NORMALIZE_FACTORS: &[(&str, u64)] = &[
        ("1m", 60),
        ("2m", 60 * 2),
        ("5m", 60 * 5),
        ("15m", 60 * 15),
        ("30m", 60 * 30),
        ("60m", 60 * 60),
        ("1mo", 86400 * 30),
        ("1wk", 86400 * 7),
        ("1d", 86400),
    ];

    fn get_normalize_factor(stock_type: &str) -> u64 {
        Self::NORMALIZE_FACTORS
            .iter()
            .find(|&&(t, _)| t == stock_type)
            .map(|&(_, factor)| factor)
            .unwrap_or(86400)
    }

    fn get_timezone_offset(lang_type: &LangType) -> (UtcOffset, u8) {
        match lang_type {
            LangType::Korean => (UtcOffset::from_hms(9, 0, 0).unwrap(), 9),
            LangType::English => {
                let month = OffsetDateTime::now_utc().month() as u8;
                match month {
                    3..=11 => (UtcOffset::from_hms(-4, 0, 0).unwrap(), 9),
                    _ => (UtcOffset::from_hms(-5, 0, 0).unwrap(), 9),
                }
            }
        }
    }

    pub fn get_latest_data(
        stock_name: &str,
        stock_type: &str,
        lang_type: &LangType,
        last_timestamp: u64,
    ) -> Option<BTreeMap<u64, StockData>> {
        let (timezone_offset, _) = Self::get_timezone_offset(lang_type);
        let provider = yahoo::YahooConnector::new().unwrap();
        let end = OffsetDateTime::now_utc().to_offset(timezone_offset);
        let normalize_factor = Self::get_normalize_factor(stock_type);
        let start = OffsetDateTime::from_unix_timestamp((last_timestamp * normalize_factor) as i64)
            .unwrap();

        // 시간 제한을 3개의 데이터에 맞게 조정
        let limit_duration = match stock_type {
            "1m" => Duration::minutes(3),
            "2m" => Duration::minutes(6),
            "5m" => Duration::minutes(15),
            "15m" => Duration::minutes(45),
            "30m" => Duration::minutes(90),
            "60m" => Duration::hours(3),
            _ => Duration::days(1),
        };

        let limited_end = (start + limit_duration).min(end);

        let quotes = match stock_type {
            "1m" | "2m" | "5m" | "15m" | "30m" | "60m" => {
                let resp = tokio_test::block_on(provider.get_quote_history_interval(
                    stock_name,
                    start,
                    limited_end,
                    stock_type,
                ));

                match resp {
                    Ok(data) => match data.quotes() {
                        Ok(mut quotes) => {
                            // 최근 3개만 유지
                            if quotes.len() > 3 {
                                quotes.truncate(3);
                            }
                            quotes
                        }
                        Err(e) => {
                            println!("Error parsing quotes: {:?}", e);
                            return None;
                        }
                    },
                    Err(e) => {
                        println!("Error getting data: {:?}", e);
                        return None;
                    }
                }
            }
            _ => {
                let resp =
                    tokio_test::block_on(provider.get_quote_range(stock_name, stock_type, "1d"));
                match resp {
                    Ok(data) => match data.quotes() {
                        Ok(mut quotes) => {
                            // 최근 3개만 유지
                            if quotes.len() > 3 {
                                quotes.truncate(3);
                            }
                            quotes
                        }
                        Err(_) => return None,
                    },
                    Err(_) => return None,
                }
            }
        };

        if quotes.is_empty() {
            println!("No new data available");
            return None;
        }

        let new_data: BTreeMap<u64, StockData> = quotes
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
            .collect();

        Some(new_data)
    }

    pub fn get_data(
        stock_name: &str,
        stock_type: &str,
        lang_type: &LangType,
    ) -> BTreeMap<u64, StockData> {
        let (timezone_offset, market_open_hour) = Self::get_timezone_offset(lang_type);
        let provider = yahoo::YahooConnector::new().unwrap();
        let end = OffsetDateTime::now_utc().to_offset(timezone_offset);

        let quotes = match stock_type {
            "1m" | "2m" | "5m" | "15m" | "30m" | "60m" => {
                let start = match lang_type {
                    LangType::Korean => end
                        .replace_hour(market_open_hour)
                        .unwrap()
                        .replace_minute(0)
                        .unwrap()
                        .replace_second(0)
                        .unwrap()
                        .replace_nanosecond(0)
                        .unwrap(),
                    LangType::English => end
                        .replace_hour(market_open_hour)
                        .unwrap()
                        .replace_minute(30)
                        .unwrap()
                        .replace_second(0)
                        .unwrap()
                        .replace_nanosecond(0)
                        .unwrap(),
                };

                let resp = tokio_test::block_on(
                    provider.get_quote_history_interval(stock_name, start, end, stock_type),
                );

                match resp {
                    Ok(data) => match data.quotes() {
                        Ok(quotes) => quotes,
                        Err(e) => {
                            println!("Error parsing quotes: {:?}", e);
                            vec![]
                        }
                    },
                    Err(e) => {
                        println!("Error getting data: {:?}", e);
                        vec![]
                    }
                }
            }
            _ => {
                let resp =
                    tokio_test::block_on(provider.get_quote_range(stock_name, stock_type, "30y"))
                        .unwrap();
                resp.quotes().unwrap()
            }
        };

        let normalize_factor = Self::get_normalize_factor(stock_type);
        let data: BTreeMap<u64, StockData> = quotes
            .into_par_iter()
            .map(|quote| {
                let candle = StockData {
                    open: quote.open as f64,
                    high: quote.high as f64,
                    low: quote.low as f64,
                    close: quote.close as f64,
                    volume: quote.volume as f64,
                };
                (quote.timestamp as u64 / normalize_factor, candle)
            })
            .collect();

        data
    }
}