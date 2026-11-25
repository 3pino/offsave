//! ユーザーのタイムゾーンから、デバイスに渡す時計の値を作る。

use jiff::Timestamp;
use jiff::tz::TimeZone;
use offsave_core::Ms;
use offsave_core::time::Clock;

pub fn valid_timezone(name: &str) -> bool {
    TimeZone::get(name).is_ok()
}

/// `now` の時点の UTC オフセットを入れた時計。タイムゾーンが読めないときは UTC。
pub fn clock_for(timezone: &str, day_boundary_min: u32, now: Ms) -> Clock {
    let offset_min = TimeZone::get(timezone)
        .ok()
        .zip(Timestamp::from_millisecond(now).ok())
        .map(|(tz, ts)| tz.to_offset(ts).seconds() / 60)
        .unwrap_or(0);
    Clock {
        utc_offset_min: offset_min,
        day_boundary_min,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokyo_is_plus_nine() {
        assert_eq!(clock_for("Asia/Tokyo", 240, 0).utc_offset_min, 540);
        assert!(valid_timezone("Asia/Tokyo"));
        assert!(!valid_timezone("Mars/Base"));
    }
}
