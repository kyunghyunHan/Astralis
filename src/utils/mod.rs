pub mod constant;
pub mod logs;

pub fn hmac_sha256(secret: &str, message: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

//거래량 조정
pub fn adjust_precision(value: f64, precision: u32) -> f64 {
    let scale = 10f64.powi(precision as i32);
    (value * scale).floor() / scale
}
