pub mod measurement_window;
pub mod stock_data;
use std::collections::BTreeMap;
use std::time::Instant;
pub mod maperiod;
pub mod time_frame;

#[derive(Clone)]
pub enum StockType {
    DAY,
    YEAR1,
}
#[derive(Debug, Clone)]
pub struct MeasurementWindow {
    pub values: BTreeMap<u64, StockData>,
    pub look_behind: usize,
    pub start_time: Instant,
    volumes: Vec<f64>, // Added volumes field
}

#[derive(Debug, Clone)]
pub struct StockData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}
#[derive(PartialEq, Clone, Debug)] // derive 속성 추가
pub enum ChartType {
    Candle,
    Line,
}
#[derive(PartialEq, Clone, Debug)] // derive 속성 추가
pub enum TimeFrame {
    Day,
    Week,
    Month,
    Year,
}
// 이동평균선 기간을 위한 enum 추가
#[derive(PartialEq, Clone, Debug)]
pub enum MAPeriod {
    MA5,
    MA10,
    MA20,
    MA60,
    MA224,
}
