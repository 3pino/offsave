//! 制限の判定。
//!
//! 入力は「ルール・オーバーライド・一時解除・今日の利用区間（全デバイス分）・今開いているもの」で、
//! 出力は「許可 / 猶予 / ブロック」と、次に判定が変わりうる時刻。副作用はない（猶予の記録は戻り値で返す）。
//!
//! 猶予: 制限に達した最初の判定で 1 分間だけ使える。同じ「制限到達イベント」では二度与えない。
//! 制限到達イベント = ルール × 発生（1 日の上限は論理日、時間帯はその時間帯の開始日）× 直近の一時解除の終了時刻。
//! 時間帯の中でアプリを開き直しても猶予は戻らない。一時解除が終わって再び制限に達したときは新しいイベントになる。
//!
//! Android の Kotlin 版は、`tests/vectors/engine.json` の同じデータで試験して、この実装と一致させる。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Ms;
use crate::intervals::{Segment, union_len};
use crate::model::{Limit, Override, Rule};
use crate::time::{Clock, DAY_MS, MIN_MS, format_date, local, logical_day};

/// 猶予の長さ。
pub const GRACE_MS: Ms = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    DailyTotal,
    Window,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Verdict {
    Allow,
    /// 猶予中。`until` までは使える。
    Grace { rule_id: String, until: Ms },
    Block { rule_id: String, kind: BlockKind },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub verdict: Verdict,
    /// 今回の判定で新しく始まった猶予（キー → 開始時刻）。呼び出し側が保存する。
    pub new_graces: Vec<(String, Ms)>,
    /// 判定が変わりうる次の時刻。無ければ `None`。
    pub next_wakeup: Option<Ms>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub rules: Vec<Rule>,
    pub overrides: Vec<Override>,
    /// 一時解除の終わり。解除していなければ 0。
    pub unlock_until: Ms,
    /// 直近の一時解除が終わった時刻（猶予のキーに使う）。無ければ 0。
    pub last_unlock_end: Ms,
    /// 今日（論理日）の利用区間。ほかのデバイス分と、今のデバイスの続きの区間を含める。
    pub segments: Vec<Segment>,
    #[serde(default)]
    pub graces: HashMap<String, Ms>,
    pub now: Ms,
    pub clock: Clock,
    /// 今前面にあるもの。
    pub current_key: Option<String>,
}

/// ルール 1 つの、今日の利用秒数（ミリ秒）。
pub fn used_ms(rule: &Rule, segments: &[Segment], day_start: Ms, day_end: Ms) -> Ms {
    union_len(
        segments
            .iter()
            .filter(|s| rule.matches(&s.key))
            .map(|s| (s.start, s.end)),
        day_start,
        day_end,
    )
}

fn overridden(rule: &Rule, overrides: &[Override], date: &str) -> bool {
    overrides
        .iter()
        .any(|o| o.date == date && o.rule_id.as_deref().is_none_or(|id| id == rule.id))
}

/// 時間帯が今当たっているか。当たっていれば、その時間帯の開始日（`YYYY-MM-DD`）。
fn window_occurrence(days: u8, start: u16, end: u16, now: Ms, clock: &Clock) -> Option<String> {
    let lt = local(now, clock);
    let (start, end) = (start as u32, end as u32);
    let has = |wd: u8| days & (1 << wd) != 0;
    if start < end {
        (has(lt.weekday) && (start..end).contains(&lt.minute)).then(|| format_date(lt.day))
    } else if start > end {
        if has(lt.weekday) && lt.minute >= start {
            Some(format_date(lt.day))
        } else if has((lt.weekday + 6) % 7) && lt.minute < end {
            Some(format_date(lt.day - 1))
        } else {
            None
        }
    } else {
        None
    }
}

/// 次に時間帯が始まる時刻（現在より後）。
fn next_window_start(days: u8, start: u16, now: Ms, clock: &Clock) -> Option<Ms> {
    let lt = local(now, clock);
    for d in 0..=7i64 {
        let day = lt.day + d;
        let wd = ((lt.weekday as i64 + d) % 7) as u8;
        if days & (1 << wd) == 0 {
            continue;
        }
        let local_start = day * DAY_MS + start as Ms * MIN_MS;
        let utc = local_start - clock.utc_offset_min as Ms * MIN_MS;
        if utc > now {
            return Some(utc);
        }
    }
    None
}

pub fn decide(input: &Input) -> Decision {
    let mut decision = Decision {
        verdict: Verdict::Allow,
        new_graces: Vec::new(),
        next_wakeup: None,
    };
    let Some(key) = input.current_key.as_deref() else {
        return decision;
    };
    let now = input.now;
    let day = logical_day(now, &input.clock);

    let mut wake: Option<Ms> = None;
    let mut wake_at = |t: Ms| wake = Some(wake.map_or(t, |w| w.min(t)));

    if now < input.unlock_until {
        wake_at(input.unlock_until);
        decision.next_wakeup = wake;
        return decision;
    }

    let mut block: Option<Verdict> = None;
    let mut grace: Option<(String, Ms)> = None;

    for rule in input
        .rules
        .iter()
        .filter(|r| r.enabled && r.matches(key))
        .filter(|r| !overridden(r, &input.overrides, &day.date))
    {
        // 制限に達していれば、その発生の識別子
        let (occurrence, kind) = match rule.limit {
            Limit::DailyTotal { seconds } => {
                let used = used_ms(rule, &input.segments, day.start, day.end);
                let limit = seconds as Ms * 1000;
                if used >= limit {
                    (Some(day.date.clone()), BlockKind::DailyTotal)
                } else {
                    wake_at(now + (limit - used));
                    (None, BlockKind::DailyTotal)
                }
            }
            Limit::Window {
                days,
                start_min,
                end_min,
            } => {
                let occ = window_occurrence(days, start_min, end_min, now, &input.clock);
                if occ.is_none()
                    && let Some(t) = next_window_start(days, start_min, now, &input.clock)
                {
                    wake_at(t);
                }
                (occ, BlockKind::Window)
            }
        };
        let Some(occurrence) = occurrence else {
            continue;
        };

        let grace_key = format!("{}|{}|{}", rule.id, occurrence, input.last_unlock_end);
        let started = match input.graces.get(&grace_key) {
            Some(t) => *t,
            None => {
                decision.new_graces.push((grace_key, now));
                now
            }
        };
        let until = started + GRACE_MS;
        if now < until {
            wake_at(until);
            if grace.as_ref().is_none_or(|(_, u)| until < *u) {
                grace = Some((rule.id.clone(), until));
            }
        } else if block.is_none() {
            block = Some(Verdict::Block {
                rule_id: rule.id.clone(),
                kind,
            });
        }
    }

    decision.verdict = match (block, grace) {
        (Some(b), _) => b,
        (None, Some((rule_id, until))) => Verdict::Grace { rule_id, until },
        (None, None) => Verdict::Allow,
    };
    decision.next_wakeup = wake;
    decision
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Target;
    use crate::time::parse_date;

    fn rule(id: &str, key: &str, limit: Limit) -> Rule {
        Rule {
            id: id.into(),
            name: id.into(),
            targets: vec![Target {
                label: id.into(),
                keys: vec![key.into()],
            }],
            limit,
            enabled: true,
        }
    }

    /// JST の 2026-11-25（水）の `h:m` を UTC のミリ秒にする。
    fn jst(h: i64, m: i64) -> Ms {
        parse_date("2026-11-25").unwrap() * DAY_MS + (h * 60 + m) * MIN_MS - 540 * MIN_MS
    }

    fn input(rules: Vec<Rule>, now: Ms, key: &str) -> Input {
        Input {
            rules,
            overrides: vec![],
            unlock_until: 0,
            last_unlock_end: 0,
            segments: vec![],
            graces: HashMap::new(),
            now,
            clock: Clock::default(),
            current_key: Some(key.into()),
        }
    }

    #[test]
    fn under_limit_allows_and_schedules_wakeup() {
        let now = jst(12, 0);
        let mut i = input(
            vec![rule("r", "android:a", Limit::DailyTotal { seconds: 600 })],
            now,
            "android:a",
        );
        i.segments = vec![Segment {
            key: "android:a".into(),
            start: now - 100_000,
            end: now,
        }];
        let d = decide(&i);
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.next_wakeup, Some(now + 500_000));
    }

    #[test]
    fn exceeding_starts_grace_then_blocks() {
        let now = jst(12, 0);
        let mut i = input(
            vec![rule("r", "android:a", Limit::DailyTotal { seconds: 60 })],
            now,
            "android:a",
        );
        i.segments = vec![Segment {
            key: "android:a".into(),
            start: now - 60_000,
            end: now,
        }];
        let d = decide(&i);
        assert_eq!(
            d.verdict,
            Verdict::Grace {
                rule_id: "r".into(),
                until: now + GRACE_MS
            }
        );
        assert_eq!(d.new_graces.len(), 1);
        // 猶予を記録して 61 秒後
        i.graces.extend(d.new_graces);
        i.now = now + 61_000;
        let d = decide(&i);
        assert_eq!(
            d.verdict,
            Verdict::Block {
                rule_id: "r".into(),
                kind: BlockKind::DailyTotal
            }
        );
        assert!(d.new_graces.is_empty());
    }

    #[test]
    fn unlock_allows_and_new_event_gets_new_grace() {
        let now = jst(12, 0);
        let mut i = input(
            vec![rule("r", "android:a", Limit::DailyTotal { seconds: 0 })],
            now,
            "android:a",
        );
        i.unlock_until = now + 900_000;
        let d = decide(&i);
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.next_wakeup, Some(now + 900_000));
        // 解除が終わったあと（last_unlock_end が変わると別のイベント）
        i.now = now + 900_001;
        i.last_unlock_end = now + 900_000;
        let d = decide(&i);
        assert!(matches!(d.verdict, Verdict::Grace { .. }));
    }

    #[test]
    fn window_across_midnight() {
        // 毎日 22:00–06:00
        let r = rule(
            "w",
            "android:a",
            Limit::Window {
                days: 0x7f,
                start_min: 1320,
                end_min: 360,
            },
        );
        let at = |h, m| input(vec![r.clone()], jst(h, m), "android:a");
        assert!(matches!(decide(&at(23, 0)).verdict, Verdict::Grace { .. }));
        assert!(matches!(decide(&at(3, 0)).verdict, Verdict::Grace { .. }));
        let d = decide(&at(12, 0));
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.next_wakeup, Some(jst(22, 0)));
    }

    #[test]
    fn window_grace_not_renewed_by_reopening() {
        let r = rule(
            "w",
            "android:a",
            Limit::Window {
                days: 0x7f,
                start_min: 1320,
                end_min: 360,
            },
        );
        let mut i = input(vec![r], jst(22, 10), "android:a");
        let d = decide(&i);
        i.graces.extend(d.new_graces);
        // 日をまたいだ 01:00 に開き直しても、同じ時間帯（11-25 開始）なので猶予は戻らない
        i.now = jst(25, 0);
        let d = decide(&i);
        assert!(matches!(d.verdict, Verdict::Block { .. }));
        assert!(d.new_graces.is_empty());
    }

    #[test]
    fn override_skips_rule_for_that_logical_day() {
        let mut i = input(
            vec![rule("r", "android:a", Limit::DailyTotal { seconds: 0 })],
            jst(12, 0),
            "android:a",
        );
        i.overrides = vec![Override {
            id: "o".into(),
            date: "2026-11-25".into(),
            rule_id: None,
        }];
        assert_eq!(decide(&i).verdict, Verdict::Allow);
        i.overrides[0].date = "2026-11-26".into();
        assert!(matches!(decide(&i).verdict, Verdict::Grace { .. }));
    }

    #[test]
    fn other_devices_count_without_double_counting() {
        let now = jst(12, 0);
        let mut i = input(
            vec![rule("r", "android:a", Limit::DailyTotal { seconds: 120 })],
            now,
            "android:a",
        );
        // 別デバイス 60 秒 + このデバイス（30 秒重なる）60 秒 → 90 秒
        i.segments = vec![
            Segment {
                key: "android:a".into(),
                start: now - 90_000,
                end: now - 30_000,
            },
            Segment {
                key: "android:a".into(),
                start: now - 60_000,
                end: now,
            },
        ];
        assert_eq!(decide(&i).verdict, Verdict::Allow);
        i.segments.push(Segment {
            key: "android:a".into(),
            start: now - 150_000,
            end: now - 90_000,
        });
        assert!(matches!(decide(&i).verdict, Verdict::Grace { .. }));
    }

    #[test]
    fn unrelated_app_is_allowed() {
        let i = input(
            vec![rule("r", "android:a", Limit::DailyTotal { seconds: 0 })],
            jst(12, 0),
            "android:b",
        );
        assert_eq!(decide(&i).verdict, Verdict::Allow);
        assert_eq!(decide(&i).next_wakeup, None);
    }
}
