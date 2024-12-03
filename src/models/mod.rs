pub mod bollinger_bands;
pub mod knn;
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
// BollingerBands 구조체와 계산 함수
pub struct BollingerBands {
    pub period: usize,
    pub num_std: f32,
    pub values: BTreeMap<u64, (f32, f32, f32)>, // (upper, middle, lower)
}
