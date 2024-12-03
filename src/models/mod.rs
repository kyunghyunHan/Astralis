pub mod knn;
pub mod bollinger_bands;
use std::collections::{BTreeMap, VecDeque};

// KNN 예측기 최적화 버전
#[derive(Debug, Clone)]
pub struct OptimizedKNNPredictor {
    pub k: usize,
    pub window_size: usize,
    pub features_buffer: VecDeque<Vec<f32>>,
    pub labels_buffer: VecDeque<bool>,
    pub buffer_size: usize,
}
// 최적화된 볼린저 밴드 구현체
pub struct OptimizedBollingerBands {
    period: usize,
    num_std: f32,
    window_size: usize,
}
