//! 日付の境界とローカル時刻。
//!
//! 「論理日」は、ローカル時刻の `day_boundary_min`（既定 4:00）から次の同時刻までの 24 時間。
//! 1 日の合計時間の制限とオーバーライド（「明日は制限なし」）は論理日で数える。
//! 時間帯の制限は、実際のカレンダー（ローカル時刻）で見る。
//! 夏時間のような途中の変化は扱わない（サーバーが送るオフセットを、接続のたびに更新する）。

use serde::{Deserialize, Serialize};

use crate::Ms;

pub const MIN_MS: Ms = 60_000;
pub const DAY_MS: Ms = 86_400_000;
pub const HOUR_MS: Ms = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    /// UTC からのずれ（分）。東京は 540。
    pub utc_offset_min: i32,
    /// 論理日の始まり（ローカルの 0 時からの分）。既定 240（4:00）。
    pub day_boundary_min: u32,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            utc_offset_min: 540,
            day_boundary_min: 240,
        }
    }
}

/// ローカルのカレンダー上の時刻。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    /// 1970-01-01 からのローカルの日数。
    pub day: i64,
    /// 曜日（0 = 月 … 6 = 日）。
    pub weekday: u8,
    /// その日の 0 時からの分。
    pub minute: u32,
}

/// 論理日。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// `YYYY-MM-DD`（論理日の始まりのローカルの日付）。
    pub date: String,
    /// その日付の曜日（0 = 月 … 6 = 日）。
    pub weekday: u8,
    /// 始まり（含む）。
    pub start: Ms,
    /// 終わり（含まない）。
    pub end: Ms,
}

pub fn local(now: Ms, clock: &Clock) -> LocalTime {
    let local = now + clock.utc_offset_min as Ms * MIN_MS;
    let day = local.div_euclid(DAY_MS);
    LocalTime {
        day,
        weekday: weekday_of(day),
        minute: (local.rem_euclid(DAY_MS) / MIN_MS) as u32,
    }
}

pub fn logical_day(now: Ms, clock: &Clock) -> Day {
    let shifted = now + (clock.utc_offset_min as Ms - clock.day_boundary_min as Ms) * MIN_MS;
    logical_day_of(shifted.div_euclid(DAY_MS), clock)
}

/// 日付（1970-01-01 からの日数）の論理日。
pub fn logical_day_of(d: i64, clock: &Clock) -> Day {
    let start = d * DAY_MS - (clock.utc_offset_min as Ms - clock.day_boundary_min as Ms) * MIN_MS;
    Day {
        date: format_date(d),
        weekday: weekday_of(d),
        start,
        end: start + DAY_MS,
    }
}

/// 1970-01-01（木）からの日数を曜日（0 = 月）にする。
pub fn weekday_of(day: i64) -> u8 {
    (day + 3).rem_euclid(7) as u8
}

/// 日数を `YYYY-MM-DD` にする（Howard Hinnant の civil_from_days）。
pub fn format_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM-DD` を日数にする。形式が違えば `None`。
pub fn parse_date(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (format_date(days) == s).then_some(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_round_trip() {
        for s in ["1970-01-01", "2024-02-29", "2026-11-25", "2000-03-01"] {
            assert_eq!(format_date(parse_date(s).unwrap()), s);
        }
        assert_eq!(parse_date("2026-02-30"), None);
        assert_eq!(parse_date("2026/02/01"), None);
        assert_eq!(format_date(0), "1970-01-01");
    }

    #[test]
    fn weekday() {
        // 1970-01-01 は木曜
        assert_eq!(weekday_of(0), 3);
        // 2026-11-25 は水曜
        assert_eq!(weekday_of(parse_date("2026-11-25").unwrap()), 2);
    }

    #[test]
    fn logical_day_before_and_after_boundary() {
        let clock = Clock::default(); // JST、4:00 境界
        // 2026-11-25 03:30 JST は、論理日では 11-24
        let at = (parse_date("2026-11-25").unwrap() * DAY_MS) + 3 * 60 * MIN_MS + 30 * MIN_MS - 540 * MIN_MS;
        let day = logical_day(at, &clock);
        assert_eq!(day.date, "2026-11-24");
        assert!(day.start <= at && at < day.end);
        // 04:00 ちょうどから 11-25
        let at = at + 30 * MIN_MS;
        let day = logical_day(at, &clock);
        assert_eq!(day.date, "2026-11-25");
        assert_eq!(day.start, at);
        assert_eq!(day.end - day.start, DAY_MS);
    }

    #[test]
    fn local_time() {
        let clock = Clock::default();
        let at = parse_date("2026-11-25").unwrap() * DAY_MS + 15 * 60 * MIN_MS; // 15:00 UTC = 翌 00:00 JST
        let lt = local(at, &clock);
        assert_eq!(format_date(lt.day), "2026-11-26");
        assert_eq!(lt.minute, 0);
        assert_eq!(lt.weekday, 3);
    }
}
