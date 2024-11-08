use crate::types::StockData;
use rayon::prelude::*;
use std::collections::BTreeMap;
use tokio_test;
use yahoo_finance_api as yahoo;
use yahoo_finance_api::time::macros::datetime;
impl StockData {
    pub fn get_data(stock_name: &str, stock_type: &str) -> BTreeMap<u64, StockData> {
        let provider = yahoo::YahooConnector::new().unwrap();
        let start = datetime!(2000-11-1 0:00:00.00 UTC);
        let end = datetime!(2024-12-31 23:59:59.99 UTC);
        let resp =
            tokio_test::block_on(provider.get_quote_history(stock_name, start, end)).unwrap();
        let quotes = resp.quotes().unwrap();

        let timestamps: Vec<u64> = quotes
            .iter()
            .map(|quote| quote.timestamp as u64 / 86400) // 일 단위로 정규화
            .collect();

        if let (Some(&first), Some(&last)) = (timestamps.first(), timestamps.last()) {
            println!("First day: {}, Last day: {}", first, last);
            println!("Total days: {}", last - first + 1);
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
                let normalized_timestamp = quote.timestamp as u64 / 86400; // 일 단위로 정규화
                (normalized_timestamp, candle)
            })
            .collect()
    }
}
