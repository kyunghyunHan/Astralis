use crate::types::TimeFrame;

impl TimeFrame {
   pub fn to_api_string(&self) -> String {
        match self {
            TimeFrame::Day => "day".to_string(),
            TimeFrame::Week => "week".to_string(),
            TimeFrame::Month => "month".to_string(),
            TimeFrame::Year => "year".to_string(),
        }
    }
}
