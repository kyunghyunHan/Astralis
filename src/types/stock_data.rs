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

        quotes
            .into_par_iter()
            .enumerate()
            .map(|(index, quote)| {
                let candle = StockData {
                    open: quote.open as f64,
                    high: quote.high as f64,
                    low: quote.low as f64,
                    close: quote.close as f64,
                    volume: quote.volume as f64,
                };
                // 간격 조정을 위해 `index`를 키로 사용
                (index as u64, candle)
            })
            .collect()
    }
}
