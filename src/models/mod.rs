pub mod account;
pub mod knn;
pub mod market;
use std::collections::VecDeque;

// KNN 예측기 최적화 버전
#[derive(Debug, Clone)]
pub struct OptimizedKNNPredictor {
    pub k: usize,
    pub window_size: usize,
    pub features_buffer: VecDeque<Vec<f32>>,
    pub labels_buffer: VecDeque<bool>,
    pub buffer_size: usize,
}
