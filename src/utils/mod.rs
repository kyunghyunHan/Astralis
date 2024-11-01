use eframe::egui::{Color32, Stroke};
use egui_plot::{Bar, BoxElem, BoxSpread};
use rayon::prelude::*;
use tokio_test;
use yahoo_finance_api as yahoo;
use yahoo_finance_api::time::macros::datetime;
use crate::types::StockType;

pub fn get_data(stock_name: &str,stock_type:&str) -> Vec<BoxElem> {
     
    let provider = yahoo::YahooConnector::new().unwrap();
    let start = datetime!(2000-1-1 0:00:00.00 UTC);
    let end = datetime!(2024-12-31 23:59:59.99 UTC);
    let resp = tokio_test::block_on(provider.get_quote_history(stock_name, start, end)).unwrap();
    let quotes = resp.quotes().unwrap();
    let red = Color32::from_rgb(255, 0, 0);
    let green = Color32::from_rgb(0, 255, 0);

    quotes
        .into_par_iter()
        .enumerate() // 인덱스와 요소를 함께 가져옴

        .map(|(index, quote)| {
            let open = quote.open as f64;
            let close = quote.close as f64;
            let volume = quote.volume as f64;
            let color = if close >= open { green } else { red };
            let timestamp = quote.timestamp as f64;
        
            /* 컬러 */
            let red = Color32::from_rgb(255, 0, 0);
            let green = Color32::from_rgb(0, 255, 0);
            let blue = Color32::from_rgb(0, 0, 255);
            /* BoxSpread 정의 */
            let lower_whisker = quote.low as f64;   // 최저가
            let upper_whisker = quote.high as f64;  // 최고가
            let lower_bound = open.min(close);       // 몸체 아래쪽 경계
            let upper_bound = open.max(close);       // 몸체 위쪽 경계
            let median = (open + close) / 2.0;       // 중앙값
            println!("lower_bound{}",lower_bound);
            println!("lower_whisker{}",lower_whisker);
            println!("upper_whisker{}",upper_whisker);
            println!("upper_bound{}",upper_bound);
            println!("median{}",median);

            let spread = BoxSpread::new(
                lower_whisker,
                lower_bound,
                median,
                upper_bound,
                upper_whisker,
            );
            // (
            //     // Bar::from(Bar {
            //     //     name: "a".to_string(),
            //     //     orientation: egui_plot::Orientation::Vertical,
            //     //     argument: timestamp,
            //     //     value: volume,
            //     //     base_offset: Some(0.),
            //     //     bar_width: 20.,
            //     //     stroke: Stroke::new(1.0, color),
            //     //     fill: color,
            //     // }),
            //     // BoxElem::new(timestamp, spread), // argument는 timestamp, spread는 위에서 정의한 값을 사용
            // )
            let color = if close >= open { blue } else { red }; // 초록색을 파란색으로 변경

            BoxElem::new(index as f64, spread)
                .whisker_width(0.0) // 수염 숨기기
                .fill(color)
                .stroke(Stroke::new(2.0, color)) // 색상 지정

            // argument는 timestamp, spread는 위에서 정의한 값을 사용
        })
        .collect()
}
