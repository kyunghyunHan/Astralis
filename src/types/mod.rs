use egui_plot::{PlotPoint, PlotPoints};
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone)]
pub enum StockType {
    DAY,
    YEAR1,
}

use std::collections::BTreeMap;
#[derive(Debug)]
pub struct MeasurementWindow {
    pub values: BTreeMap<u64, f64>,
    pub look_behind: usize,
    pub start_time: Instant,
}
impl MeasurementWindow {
    pub fn new_with_look_behind(look_behind: usize, data: BTreeMap<u64, f64>) -> Self {
        Self {
            values: data,
            look_behind,
            start_time: Instant::now(),
        }
    }

    pub fn add(&mut self, x: f64, y: f64) {
        // 현재 시간
        let now = Instant::now();

        // 기준 시간 계산
        let limit_time = now - Duration::from_secs(self.look_behind as u64);

        // 오래된 값 제거
        self.values.retain(|&key, _| {
            let timestamp = Instant::now() - Duration::from_secs(key);
            timestamp >= limit_time
        });

        // 측정값 추가 (동일한 x 값이 있을 경우 대체됨)
        self.values.insert(x as u64, y);
    }

    pub fn plot_values(&self) -> PlotPoints {
        // BTreeMap에서 y 값을 추출하여 Vec<(f64, f64)> 형태로 변환
        let points: Vec<PlotPoint> = self
            .values
            .iter()
            .map(|(key, &value)| PlotPoint {
                x: *key as f64,
                y: value,
            }) // Float64를 f64로 변환
            .collect();

        // PlotPoints로 변환하여 반환
        egui_plot::PlotPoints::Owned(points)
    }
}
