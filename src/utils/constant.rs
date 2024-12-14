use iced::Color;

//Momentum

pub const MOMENTUM_1MINUTE_PERIOD: usize = 8; //Period가 클수록 장기 추세 증기
pub const MOMENTUM_1MINUTE_THRESHOLD: f32 = 0.3; //클수록 강한 변화만 포착
pub const MOMENTUM_1MINUTE_VOLUME_THRESHOLD: f32 = 1.2; //클수록 거래량이 많은 경우만 포착

pub const MOMENTUM_3MINUTE_PERIOD: usize = 6; //Period가 클수록 장기 추세 증기
pub const MOMENTUM_3MINUTE_THRESHOLD: f32 = 0.5; //클수록 강한 변화만 포착
pub const MOMENTUM_3MINUTE_VOLUME_THRESHOLD: f32 = 1.25; //클수록 거래량이 많은 경우만 포착

pub const MOMENTUM_DAY_PERIOD: usize = 10; //Period가 클수록 장기 추세 증기
pub const MOMENTUM_DAY_THRESHOLD: f32 = 2.0; //클수록 강한 변화만 포착
pub const MOMENTUM_DAY_VOLUME_THRESHOLD: f32 = 1.2; //클수록 거래량이 많은 경우만 포착

//COLOR
pub const BRIGHT_RED: Color = Color::from_rgb(0.7, 0.1, 0.1);
pub const DAKR_RED: Color = Color::from_rgb(0.7, 0.1, 0.1);

pub const BRIGHT_BLUE: Color = Color::from_rgb(0.1, 0.3, 0.7);
pub const DAKR_BLUE: Color = Color::from_rgb(0.3, 0.5, 0.8);

pub const ORNAGE: Color = Color::from_rgb(1.0, 0.647, 0.0);
pub const BRIGH_GREEN: Color = Color::from_rgb(0.0, 0.8, 0.);

pub const MIDIUM_GREEN: Color = Color::from_rgb(0.5, 0.5, 0.5);
pub const BUTTON_ROUND: f32 = 10.;
pub const YELLOW: Color = Color::from_rgb(1.0, 1.0, 0.0);
//window

pub const WINDOW_WIDTH: f32 = 1980.;
pub const WINDOW_HIGHT: f32 = 1080.;

pub static DEFAULT_ARR: [&str; 14] = [
    "BTC", "ETH", "XRP", "SOL", "DOT", "TRX", "TON", "SHIB", "DOGE", "PEPE", "BNB", "SUI", "XLM",
    "ADA",
];

//address

pub static BINANCE_FAPI_ADDRESS: &str = "https://fapi.binance.com/fapi/v1";
pub static BINANCE_FWSS_ADDRESS: &str = "wss://fstream.binance.com/ws";
