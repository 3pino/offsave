//! 区間の和集合。複数のデバイスで同時に使った時間を、重複なく数える。

use serde::{Deserialize, Serialize};

use crate::Ms;

/// 前面にあったアプリ・サイトの区間 `[start, end)`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// 対象の識別子。`android:com.example.app` / `win:game.exe` / `linux:game.desktop` / `domain:example.com`。
    pub key: String,
    pub start: Ms,
    pub end: Ms,
}

/// `[lo, hi)` に切って重ね、和集合の長さ（ミリ秒）を返す。
pub fn union_len(ranges: impl IntoIterator<Item = (Ms, Ms)>, lo: Ms, hi: Ms) -> Ms {
    merge(ranges.into_iter().filter_map(|(s, e)| {
        let (s, e) = (s.max(lo), e.min(hi));
        (s < e).then_some((s, e))
    }))
    .into_iter()
    .map(|(s, e)| e - s)
    .sum()
}

/// 重なる・接する区間をまとめ、開始の昇順にする。
pub fn merge(ranges: impl IntoIterator<Item = (Ms, Ms)>) -> Vec<(Ms, Ms)> {
    let mut v: Vec<(Ms, Ms)> = ranges.into_iter().filter(|(s, e)| s < e).collect();
    v.sort_unstable();
    let mut out: Vec<(Ms, Ms)> = Vec::with_capacity(v.len());
    for (s, e) in v {
        match out.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => out.push((s, e)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_devices_are_not_double_counted() {
        // 端末 A: 0–100、端末 B: 50–150 → 150
        assert_eq!(union_len([(0, 100), (50, 150)], 0, 1000), 150);
    }

    #[test]
    fn clipped_to_day() {
        assert_eq!(union_len([(-50, 50), (90, 200)], 0, 100), 60);
    }

    #[test]
    fn touching_ranges_merge() {
        assert_eq!(merge([(0, 10), (10, 20), (30, 40)]), vec![(0, 20), (30, 40)]);
    }

    #[test]
    fn empty_and_inverted_ranges_are_ignored() {
        assert_eq!(union_len([(5, 5), (9, 3)], 0, 100), 0);
    }
}
