use crate::models::OptimizedBollingerBands;
use crate::Candlestick;


impl OptimizedBollingerBands {
    pub fn new(period: usize, num_std: f32, window_size: usize) -> Self {
        Self {
            period,
            num_std,
            window_size,
        }
    }

    pub fn calculate_bands(&self, candlesticks: &[(&u64, &Candlestick)]) -> Option<(f32, f32, f32)> {
        if candlesticks.len() < self.window_size {
            return None;
        }

        let window = &candlesticks[candlesticks.len() - self.window_size..];
        
        // 중간 밴드(SMA) 계산
        let sma = window.iter()
            .map(|(_, c)| c.close)
            .sum::<f32>() / self.window_size as f32;
        
        // 표준편차 계산
        let variance = window.iter()
            .map(|(_, c)| {
                let diff = c.close - sma;
                diff * diff
            })
            .sum::<f32>() / self.window_size as f32;
        
        let std_dev = variance.sqrt();
        
        Some((
            sma + (self.num_std * std_dev), // 상단 밴드
            sma,                            // 중간 밴드
            sma - (self.num_std * std_dev), // 하단 밴드
        ))
    }
}