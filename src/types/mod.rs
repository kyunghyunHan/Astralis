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
    volumes: Vec<f64>, // Added volumes field
}
impl MeasurementWindow {
    pub fn new_with_look_behind(look_behind: usize, data: BTreeMap<u64, f64>) -> Self {
        Self {
            values: data,
            look_behind,
            start_time: Instant::now(),
            volumes: Vec::new(), // Initialize volumes
        }
    }

    pub fn add(&mut self, x: f64, y: f64) {
        let now = Instant::now();
        let limit_time = now - Duration::from_secs(self.look_behind as u64);

        // Remove old values
        self.values.retain(|&key, _| {
            let timestamp = self.start_time + Duration::from_secs(key);
            timestamp >= limit_time
        });

        // Add new value
        self.values.insert(x as u64, y);
    }

    pub fn plot_values(&self) -> PlotPoints {
        let points: Vec<PlotPoint> = self
            .values
            .iter()
            .map(|(&key, &value)| PlotPoint { x: key as f64, y: value })
            .collect();

        PlotPoints::Owned(points)
    }

    pub fn volumes(&self) -> &Vec<f64> {
        &self.volumes
    }

    pub fn high_price(&self) -> Option<f64> {
        self.values.values().copied().max_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn low_price(&self) -> Option<f64> {
        self.values.values().copied().min_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    // Helper method to get points as Vec for iteration
    pub fn get_points(&self) -> Vec<PlotPoint> {
        self.values
            .iter()
            .map(|(&key, &value)| PlotPoint { x: key as f64, y: value })
            .collect()
    }

    // Add method to update volumes
    pub fn add_volume(&mut self, volume: f64) {
        self.volumes.push(volume);
    }
}