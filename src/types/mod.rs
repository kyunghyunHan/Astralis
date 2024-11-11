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
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeFrame {
    Minute1,    // 1분봉
    Minute2,    // 2분봉
    Minute5,    // 5분봉
    Minute15,   // 15분봉
    Minute30,   // 30분봉
    Hour1,      // 1시간봉
    Day,        // 일봉
    Week,       // 주봉
    Month,      // 월봉
}



// 이동평균선 기간을 위한 enum 추가
// MAPeriod 열거형에 필요한 derive 매크로 추가
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum MAPeriod {
    MA5,
    MA10,
    MA20,
    MA60,
    MA224,
}
