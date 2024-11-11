use crate::types::TimeFrame;

impl TimeFrame {
    pub fn to_string(&self) -> &str {
        match self {
            TimeFrame::Minute1 => "1m",
            TimeFrame::Minute2 => "2m",
            TimeFrame::Minute5 => "5m",
            TimeFrame::Minute15 => "15m",
            TimeFrame::Minute30 => "30m",
            TimeFrame::Hour1 => "60m",
            TimeFrame::Day => "1d",
            TimeFrame::Week => "1wk",
            TimeFrame::Month => "1mo",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TimeFrame::Minute1 => "1분",
            TimeFrame::Minute2 => "2분",
            TimeFrame::Minute5 => "5분",
            TimeFrame::Minute15 => "15분",
            TimeFrame::Minute30 => "30분",
            TimeFrame::Hour1 => "1시간",
            TimeFrame::Day => "일봉",
            TimeFrame::Week => "주봉",
            TimeFrame::Month => "월봉",
        }
    }
    pub fn all_timeframes() -> Vec<TimeFrame> {
        vec![
            TimeFrame::Day,
            TimeFrame::Week,
            TimeFrame::Month,
            TimeFrame::Hour1,
            TimeFrame::Minute1,
            TimeFrame::Minute2,
            TimeFrame::Minute5,
            TimeFrame::Minute30,
            TimeFrame::Minute30,
        ]
    }
}
