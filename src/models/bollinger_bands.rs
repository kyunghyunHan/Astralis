use crate::models::BollingerBands;
use crate::Candlestick;
use std::collections::BTreeMap;

impl BollingerBands {
    pub fn new(period: usize, num_std: f32) -> Self {
        Self {
            period,
            num_std,
            values: BTreeMap::new(),
        }
    }

    pub fn calculate(&mut self, candlesticks: &BTreeMap<u64, Candlestick>) {
        self.values.clear();

        if candlesticks.len() < self.period {
            return;
        }

        let data: Vec<(&u64, &Candlestick)> = candlesticks.iter().collect();

        for i in (self.period - 1)..data.len() {
            let window = &data[(i + 1 - self.period)..=i];

            // 중간 밴드 (SMA) 계산
            let sum: f32 = window.iter().map(|(_, c)| c.close).sum();
            let sma = sum / self.period as f32;

            // 표준편차 계산
            let variance = window
                .iter()
                .map(|(_, c)| {
                    let diff = c.close - sma;
                    diff * diff
                })
                .sum::<f32>()
                / self.period as f32;

            let std_dev = variance.sqrt();

            // 밴드 계산
            let upper = sma + (self.num_std * std_dev);
            let lower = sma - (self.num_std * std_dev);

            self.values.insert(*data[i].0, (upper, sma, lower));
        }
    }
}
