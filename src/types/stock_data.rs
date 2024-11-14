use crate::types::StockData;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use time::Duration;
use time::{OffsetDateTime, UtcOffset};
use tokio_test;
use yahoo_finance_api as yahoo;

use super::LangType;

// 상수들을 static으로 캐싱
static NORMALIZE_FACTORS: Lazy<HashMap<&'static str, u64>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("1m", 60);
    map.insert("2m", 60 * 2);
    map.insert("5m", 60 * 5);
    map.insert("15m", 60 * 15);
    map.insert("30m", 60 * 30);
    map.insert("60m", 60 * 60);
    map.insert("1mo", 86400 * 30);
    map.insert("1wk", 86400 * 7);
    map.insert("1d", 86400);
    map
});

// Yahoo provider를 재사용하기 위한 static
static PROVIDER: Lazy<yahoo::YahooConnector> = Lazy::new(|| yahoo::YahooConnector::new().unwrap());

impl StockData {
    #[inline]
    fn get_normalize_factor(stock_type: &str) -> u64 {
        *NORMALIZE_FACTORS.get(stock_type).unwrap_or(&86400)
    }

    #[inline]
    fn get_timezone_offset(lang_type: &LangType) -> (UtcOffset, u8) {
        match lang_type {
            LangType::Korean => (UtcOffset::from_hms(9, 0, 0).unwrap(), 9),
            LangType::English => {
                let month = OffsetDateTime::now_utc().month() as u8;
                if (3..=11).contains(&month) {
                    (UtcOffset::from_hms(-4, 0, 0).unwrap(), 9)
                } else {
                    (UtcOffset::from_hms(-5, 0, 0).unwrap(), 9)
                }
            }
        }
    }

    fn get_limit_duration(stock_type: &str) -> Duration {
        match stock_type {
            "1m" => Duration::minutes(3),
            "2m" => Duration::minutes(6),
            "5m" => Duration::minutes(15),
            "15m" => Duration::minutes(45),
            "30m" => Duration::minutes(90),
            "60m" => Duration::hours(3),
            _ => Duration::days(1),
        }
    }

    pub fn get_latest_data(
        stock_name: &str,
        stock_type: &str,
        lang_type: &LangType,
        last_timestamp: u64,
    ) -> Option<BTreeMap<u64, StockData>> {
        let (timezone_offset, _) = Self::get_timezone_offset(lang_type);
        let end = OffsetDateTime::now_utc().to_offset(timezone_offset);
        let normalize_factor = Self::get_normalize_factor(stock_type);
        let start = OffsetDateTime::from_unix_timestamp((last_timestamp * normalize_factor) as i64)
            .unwrap();

        let limited_end = (start + Self::get_limit_duration(stock_type)).min(end);

        let quotes = if matches!(stock_type, "1m" | "2m" | "5m" | "15m" | "30m" | "60m") {
            tokio_test::block_on(PROVIDER.get_quote_history_interval(
                stock_name,
                start,
                limited_end,
                stock_type,
            ))
            .ok()
            .and_then(|data| data.quotes().ok())
        } else {
            tokio_test::block_on(PROVIDER.get_quote_range(stock_name, stock_type, "1d"))
                .ok()
                .and_then(|data| data.quotes().ok())
        };

        quotes.map(|mut quotes| {
            quotes.truncate(3);
            quotes
                .into_par_iter()
                .map(|quote| {
                    (
                        quote.timestamp as u64 / normalize_factor,
                        StockData {
                            open: quote.open as f64,
                            high: quote.high as f64,
                            low: quote.low as f64,
                            close: quote.close as f64,
                            volume: quote.volume as f64,
                        },
                    )
                })
                .collect()
        })
    }

    pub fn get_data(
        stock_name: &str,
        stock_type: &str,
        lang_type: &LangType,
    ) -> BTreeMap<u64, StockData> {
        let (timezone_offset, market_open_hour) = Self::get_timezone_offset(lang_type);
        let end = OffsetDateTime::now_utc().to_offset(timezone_offset);

        let quotes = if matches!(stock_type, "1m" | "2m" | "5m" | "15m" | "30m" | "60m") {
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

            tokio_test::block_on(
                PROVIDER.get_quote_history_interval(stock_name, start, end, stock_type),
            )
            .ok()
            .and_then(|data| data.quotes().ok())
            .unwrap_or_default()
        } else {
            tokio_test::block_on(PROVIDER.get_quote_range(stock_name, stock_type, "30y"))
                .ok()
                .and_then(|data| data.quotes().ok())
                .unwrap_or_default()
        };

        let normalize_factor = Self::get_normalize_factor(stock_type);
        quotes
            .into_par_iter()
            .map(|quote| {
                (
                    quote.timestamp as u64 / normalize_factor,
                    StockData {
                        open: quote.open as f64,
                        high: quote.high as f64,
                        low: quote.low as f64,
                        close: quote.close as f64,
                        volume: quote.volume as f64,
                    },
                )
            })
            .collect()
    }
}
