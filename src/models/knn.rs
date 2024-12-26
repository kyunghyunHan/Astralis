use crate::models::OptimizedKNNPredictor;
use crate::Candlestick;
use std::collections::VecDeque;

/*
KNN 예측 지표
*/
impl OptimizedKNNPredictor {
    pub fn new(k: usize, window_size: usize, buffer_size: usize) -> Self {
        Self {
            k,
            window_size,
            features_buffer: VecDeque::with_capacity(buffer_size),
            labels_buffer: VecDeque::with_capacity(buffer_size),
        }
    }

    // 특성 추출 최적화:가격 데이터 로부터 예측에 사용할 특성들을 추출
    //1.MA(이동평균)비율
    //2.RSI(상대 강도 지수)
    //3.거래량 비율
    //4.최근 가격 변화율
    pub fn extract_features(&self, candlesticks: &[(&u64, &Candlestick)]) -> Option<Vec<f32>> {
        if candlesticks.len() < self.window_size {
            return None;
        }

        let mut features = Vec::with_capacity(self.window_size * 4);

        // 가격 변화율 계산
        let mut price_changes = Vec::with_capacity(self.window_size - 1);
        for window in candlesticks.windows(2) {
            let price_change =
                ((window[1].1.close - window[0].1.close) / window[0].1.close) * 100.0;
            price_changes.push(price_change);
        }

        // 기술적 지표 계산
        let (ma5, ma20) = self.calculate_moving_averages(candlesticks);
        let rsi = self.calculate_rsi(&price_changes, 14);
        let volume_ratio = self.calculate_volume_ratio(candlesticks);

        // 특성 결합
        features.extend_from_slice(&[
            ma5 / ma20 - 1.0,                             // MA 비율
            rsi / 100.0,                                  // 정규화된 RSI
            volume_ratio,                                 // 거래량 비율
            price_changes.last().unwrap_or(&0.0) / 100.0, // 최근 가격 변화
        ]);

        Some(features)
    }

    // 이동평균 계산 최적화:5일 20일 이동 평균 계산
    pub fn calculate_moving_averages(&self, data: &[(&u64, &Candlestick)]) -> (f32, f32) {
        let (ma5, ma20) = data
            .iter()
            .rev()
            .take(20)
            .fold((0.0, 0.0), |acc, (_, candle)| {
                (
                    if data.len() >= 5 {
                        acc.0 + candle.close / 5.0
                    } else {
                        acc.0
                    },
                    acc.1 + candle.close / 20.0,
                )
            });
        (ma5, ma20)
    }

    // RSI 계산 최적화
    //period:기준 날짜
    pub fn calculate_rsi(&self, price_changes: &[f32], period: usize) -> f32 {
        let (gains, losses): (Vec<_>, Vec<_>) = price_changes
            .iter()
            .map(|&change| {
                if change > 0.0 {
                    (change, 0.0)
                } else {
                    (0.0, -change)
                }
            })
            .unzip();

        let avg_gain: f32 = gains.iter().sum::<f32>() / period as f32;
        let avg_loss: f32 = losses.iter().sum::<f32>() / period as f32;

        if avg_loss == 0.0 {
            100.0
        } else {
            100.0 - (100.0 / (1.0 + (avg_gain / avg_loss)))
        }
    }

    // 거래량 비율 계산
    pub fn calculate_volume_ratio(&self, data: &[(&u64, &Candlestick)]) -> f32 {
        let recent_volume = data.last().map(|(_, c)| c.volume).unwrap_or(0.0);
        let avg_volume = data.iter().map(|(_, c)| c.volume).sum::<f32>() / data.len() as f32;
        recent_volume / avg_volume
    }

    // 예측 최적화
    pub fn predict(&self, features: &[f32]) -> Option<String> {
        if self.features_buffer.is_empty() {
            return None;
        }

        let mut distances: Vec<(f32, bool)> = self
            .features_buffer
            .iter()
            .zip(self.labels_buffer.iter())
            .map(|(train_features, &label)| {
                let distance = self.euclidean_distance(features, train_features);
                (distance, label)
            })
            .collect();

        distances.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let up_votes = distances
            .iter()
            .take(self.k)
            .filter(|&&(_, label)| label)
            .count();

        Some(if up_votes > self.k / 2 { "▲" } else { "▼" }.to_string())
    }

    // 거리 계산 최적화
    pub fn euclidean_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }
}
