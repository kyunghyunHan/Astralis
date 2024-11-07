use crate::types::MAPeriod;

impl MAPeriod {
    pub fn value(&self) -> usize {
        match self {
            MAPeriod::MA5 => 5,
            MAPeriod::MA10 => 10,
            MAPeriod::MA20 => 20,
            MAPeriod::MA60 => 60,
            MAPeriod::MA224 => 224,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            MAPeriod::MA5 => "5MA",
            MAPeriod::MA10 => "10MA",
            MAPeriod::MA20 => "20MA",
            MAPeriod::MA60 => "60MA",
            MAPeriod::MA224 => "224MA",
        }
    }
}
