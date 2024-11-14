use crate::types::MAPeriod;
use eframe::egui;
impl MAPeriod {
    pub fn get_color(&self) -> egui::Color32 {
        match self {
            MAPeriod::MA5 => egui::Color32::from_rgb(255, 0, 0),  // 빨강
            MAPeriod::MA10 => egui::Color32::from_rgb(0, 255, 0), // 초록
            MAPeriod::MA20 => egui::Color32::from_rgb(0, 0, 255), // 파랑
            MAPeriod::MA60 => egui::Color32::from_rgb(255, 165, 0), // 주황
            MAPeriod::MA224 => egui::Color32::from_rgb(128, 0, 128), // 보라
        }
    }

    // 기존의 value() 메서드도 같이 보여드립니다
    pub fn value(&self) -> usize {
        match self {
            MAPeriod::MA5 => 5,
            MAPeriod::MA10 => 10,
            MAPeriod::MA20 => 20,
            MAPeriod::MA60 => 60,
            MAPeriod::MA224 => 224,
        }
    }

    pub fn name(&self) -> String {
        match self {
            MAPeriod::MA5 => "MA5".to_string(),
            MAPeriod::MA10 => "MA10".to_string(),
            MAPeriod::MA20 => "MA20".to_string(),
            MAPeriod::MA60 => "MA60".to_string(),
            MAPeriod::MA224 => "MA224".to_string(),
        }
    }
}