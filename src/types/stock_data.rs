use crate::types::StockData;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;
use time::Duration;
use time::{OffsetDateTime, UtcOffset};
use tokio_test;
use yahoo_finance_api as yahoo;

use super::LangType;

// 캐시를 위한 타입 정의
type CacheKey = (String, String, LangType);
type CacheValue = (OffsetDateTime, BTreeMap<u64, StockData>);

// 데이터 캐싱을 위한 static
static CACHE: Lazy<Mutex<HashMap<CacheKey, CacheValue>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// 상수들을 static으로 캐싱 (미리 계산된 값 사용)
static NORMALIZE_FACTORS: Lazy<HashMap<&'static str, u64>> = Lazy::new(|| {
    let mut map = HashMap::with_capacity(9);
    map.insert("1m", 60);
    map.insert("2m", 120);
    map.insert("5m", 300);
    map.insert("15m", 900);
    map.insert("30m", 1800);
    map.insert("60m", 3600);
    map.insert("1mo", 2_592_000);
    map.insert("1wk", 604_800);
    map.insert("1d", 86400);
    map
});

// 자주 사용되는 stock type 체크를 위한 상수 집합
static MINUTE_INTERVALS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::with_capacity(6);
    set.insert("1m");
    set.insert("2m");
    set.insert("5m");
    set.insert("15m");
    set.insert("30m");
    set.insert("60m");
    set
});

// Duration 매핑을 위한 상수
static DURATION_MAP: Lazy<HashMap<&'static str, Duration>> = Lazy::new(|| {
    let mut map = HashMap::with_capacity(7);
    map.insert("1m", Duration::minutes(3));
    map.insert("2m", Duration::minutes(6));
    map.insert("5m", Duration::minutes(15));
    map.insert("15m", Duration::minutes(45));
    map.insert("30m", Duration::minutes(90));
    map.insert("60m", Duration::hours(3));
    map.insert("1d", Duration::days(1));
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

    #[inline]
    fn get_limit_duration(stock_type: &str) -> Duration {
        DURATION_MAP
            .get(stock_type)
            .copied()
            .unwrap_or(Duration::days(1))
    }

    fn transform_quote(quote: yahoo::Quote, normalize_factor: u64) -> (u64, StockData) {
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

        let quotes = if MINUTE_INTERVALS.contains(stock_type) {
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
                .map(|quote| Self::transform_quote(quote, normalize_factor))
                .collect()
        })
    }

    pub fn get_data(
        stock_name: &str,
        stock_type: &str,
        lang_type: &LangType,
    ) -> BTreeMap<u64, StockData> {
        let cache_key = (
            stock_name.to_string(),
            stock_type.to_string(),
            lang_type.clone(),
        );

        // 캐시 확인
        if let Some(cache) = CACHE.lock().unwrap().get(&cache_key) {
            let (cache_time, data) = cache;
            let age = OffsetDateTime::now_utc() - *cache_time;
            
            // 캐시가 5분 이내면 캐시된 데이터 반환
            if age < Duration::minutes(5) {
                return data.clone();
            }
        }

        let (timezone_offset, market_open_hour) = Self::get_timezone_offset(lang_type);
        let end = OffsetDateTime::now_utc().to_offset(timezone_offset);

        let quotes = if MINUTE_INTERVALS.contains(stock_type) {
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
        let result: BTreeMap<u64, StockData> = quotes
            .into_par_iter()
            .map(|quote| Self::transform_quote(quote, normalize_factor))
            .collect();

        // 결과를 캐시에 저장
        CACHE.lock().unwrap().insert(
            cache_key,
            (OffsetDateTime::now_utc(), result.clone()),
        );

        result
    }
}