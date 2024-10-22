use eframe::egui::Color32;

pub fn colors(open: f64, close: f64) -> Color32 {
    if close >= open {
        Color32::from_rgb(235, 52, 52)
    // 상승봉 빨간색
    } else {
        Color32::from_rgb(71, 135, 231)
        // 하락봉 파란색
    }
}
