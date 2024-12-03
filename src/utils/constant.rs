use iced::Color;

//Momentum
pub const MOMENTUM_DAY_PERIOD: usize = 5; //Period가 클수록 장기 추세 증기
pub const MOMENTUM_DAY_THRESHOLD: f32 = 5.; //클수록 강한 변화만 포착
pub const MOMENTUM_DAY_VOLUME_THRESHOLD: f32 = 5.; //클수록 거래량이 많은 경우만 포착

pub const MOMENTUM_3MINUTE_PERIOD: usize = 5; //Period가 클수록 장기 추세 증기
pub const MOMENTUM_3MINUTE_THRESHOLD: f32 = 5.; //클수록 강한 변화만 포착
pub const MOMENTUM_3MINUTE_VOLUME_THRESHOLD: f32 = 5.; //클수록 거래량이 많은 경우만 포착

pub const MOMENTUM_1MINUTE_PERIOD: usize = 5; //Period가 클수록 장기 추세 증기
pub const MOMENTUM_1MINUTE_THRESHOLD: f32 = 5.; //클수록 강한 변화만 포착
pub const MOMENTUM_1MINUTE_VOLUME_THRESHOLD: f32 = 5.; //클수록 거래량이 많은 경우만 포착
                                                       //COLOR
pub const BRIGHT_RED: Color = Color::from_rgb(0.7, 0.1, 0.1);
pub const DAKR_RED: Color = Color::from_rgb(0.7, 0.1, 0.1);
