pub mod measurement_window;
pub mod stock_data;
pub mod stocki;
use std::collections::BTreeMap;
pub mod maperiod;
pub mod time_frame;
use eframe::egui::{self, Id, RichText, Vec2b};
use std::collections::HashMap;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

// 신호 유형 정의
#[derive(Debug, Clone, Copy)]
enum SignalType {
    Buy,     // 매수 신호 (골든 크로스)
    Sell,    // 매도 신호 (데드 크로스)
    BuyRSI,  // RSI 기반 매수 신호
    SellRSI, // RSI 기반 매도 신호
}
pub struct Stocki {
    pub measurements: Arc<Mutex<MeasurementWindow>>,
    pub selected_stock: String, // Arc<Mutex> 제거
    pub stocks: Vec<String>,
    pub chart_type: ChartType,
    pub lang_type: LangType,
    pub time_frame: TimeFrame,
    pub previous_time_frame: TimeFrame, // 추가
    pub ma_states: HashMap<MAPeriod, bool>,
    pub chart_id: Id,
    pub volume_id: Id,
    pub rsi_id: Id,
    pub  last_update: Instant,
}
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
    volumes: Vec<f64>,
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
pub enum LangType {
    English,
    Korean,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeFrame {
    Minute1,  // 1분봉
    Minute2,  // 2분봉
    Minute5,  // 5분봉
    Minute15, // 15분봉
    Minute30, // 30분봉
    Hour1,    // 1시간봉
    Day,      // 일봉
    Week,     // 주봉
    Month,    // 월봉
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
