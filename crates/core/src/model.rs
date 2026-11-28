//! ルールなどのデータ型。

use serde::{Deserialize, Serialize};

/// 制限の対象 1 つ。OS ごとの識別子を同じ名前にまとめる（Discord = Android と Windows の両方）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub label: String,
    pub keys: Vec<String>,
}

/// 曜日のビット（bit0 = 月 … bit6 = 日）の全部。
pub const ALL_DAYS: u8 = 0x7f;

fn all_days() -> u8 {
    ALL_DAYS
}

/// 制限 1 つ。ルールは複数の制限を持てて、どれか 1 つでも当たっていれば使えない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Limit {
    /// 使えない時間帯。`days` は開始する曜日のビット（bit0 = 月 … bit6 = 日）。
    /// `start_min > end_min` は日をまたぐ（例: 22:00–06:00 は 1320 と 360）。`end_min` は 1440 まで。
    Window {
        days: u8,
        start_min: u16,
        end_min: u16,
    },
    /// 1 日（論理日）の合計の上限（秒）。`days` はその論理日の曜日（平日は 3 時間・休日は 1 時間、のように分けられる）。
    DailyTotal {
        #[serde(default = "all_days")]
        days: u8,
        seconds: u32,
    },
}

/// 1 つのルールに置ける制限の数の上限。
pub const MAX_LIMITS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub targets: Vec<Target>,
    pub limits: Vec<Limit>,
    pub enabled: bool,
}

/// 論理日 1 日だけ、ルールを外す（「明日は制限なし」）。`rule_id` が無ければ全ルール。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Override {
    pub id: String,
    pub date: String,
    pub rule_id: Option<String>,
}

/// 識別子が対象に当たるか。`domain:` はサブドメインも含む。
pub fn key_matches(target: &str, key: &str) -> bool {
    if target == key {
        return true;
    }
    match (target.strip_prefix("domain:"), key.strip_prefix("domain:")) {
        (Some(t), Some(k)) => k.ends_with(&format!(".{t}")),
        _ => false,
    }
}

/// 2 つの識別子が、同じものを指しうるか（`domain:` は、どちらかがもう一方のサブドメインでも重なる）。
/// 1 つのアプリ・サイトは 1 つのルールにしか入れられないので、その判定に使う。
pub fn keys_overlap(a: &str, b: &str) -> bool {
    key_matches(a, b) || key_matches(b, a)
}

impl Rule {
    /// 画面や通知に出す名前。名前が空なら、対象から作る（「YouTube」「YouTube ほか3件」）。
    /// TypeScript の `ruleDisplayName`（app/src/lib/limits.ts）と同じ。
    pub fn display_name(&self) -> String {
        let name = self.name.trim();
        if !name.is_empty() {
            return name.to_owned();
        }
        match self.targets.as_slice() {
            [] => "ルール".to_owned(),
            [t] => t.label.clone(),
            [t, rest @ ..] => format!("{} ほか{}件", t.label, rest.len()),
        }
    }

    pub fn matches(&self, key: &str) -> bool {
        self.targets
            .iter()
            .any(|t| t.keys.iter().any(|k| key_matches(k, key)))
    }

    /// 保存してよい形か。だめなら理由を返す。
    pub fn validate(&self) -> Result<(), String> {
        // 名前は空でよい（空なら、対象から作った名前を出す）
        if self.name.chars().count() > 60 {
            return Err("ルール名は 60 文字以内にしてください".into());
        }
        if self.targets.is_empty() || self.targets.len() > 50 {
            return Err("対象は 1〜50 個にしてください".into());
        }
        for t in &self.targets {
            if t.label.trim().is_empty() || t.label.chars().count() > 60 {
                return Err("対象の名前は 1〜60 文字にしてください".into());
            }
            if t.keys.is_empty() || t.keys.len() > 10 {
                return Err("対象の識別子は 1〜10 個にしてください".into());
            }
            for k in &t.keys {
                if !valid_key(k) {
                    return Err(format!("識別子が不正です: {k}"));
                }
            }
        }
        if self.limits.is_empty() || self.limits.len() > MAX_LIMITS {
            return Err(format!("制限は 1〜{MAX_LIMITS} 個にしてください"));
        }
        for limit in &self.limits {
            match *limit {
                Limit::Window {
                    days,
                    start_min,
                    end_min,
                } => {
                    if days == 0 || days > ALL_DAYS {
                        return Err("曜日を 1 つ以上選んでください".into());
                    }
                    if start_min >= 1440 || end_min == 0 || end_min > 1440 || start_min == end_min {
                        return Err("時間帯が不正です".into());
                    }
                }
                Limit::DailyTotal { days, seconds } => {
                    if days == 0 || days > ALL_DAYS {
                        return Err("曜日を 1 つ以上選んでください".into());
                    }
                    if seconds > 86_400 {
                        return Err("1 日の上限は 24 時間までです".into());
                    }
                }
            }
        }
        Ok(())
    }
}

impl Limit {
    /// `self` は `old` と同じか、それより厳しいか（`old` が当たるときは必ず `self` も当たるか）。
    /// 同じ種類どうしだけを比べる（種類をまたいだ置き換えは、緩めるかどうかを判断しない）。
    pub fn is_at_least_as_strict_as(&self, old: &Limit) -> bool {
        match (self, old) {
            (
                Limit::DailyTotal { days, seconds },
                Limit::DailyTotal {
                    days: old_days,
                    seconds: old_seconds,
                },
            ) => days & old_days == *old_days && seconds <= old_seconds,
            (
                Limit::Window {
                    days,
                    start_min,
                    end_min,
                },
                Limit::Window {
                    days: old_days,
                    start_min: old_start,
                    end_min: old_end,
                },
            ) => {
                // 日をまたぐ時間帯は終わりを +24 時間して、「開始の日からの範囲」で比べる
                let span = |s: u16, e: u16| (s as u32, if e > s { e as u32 } else { e as u32 + 1440 });
                let (s, e) = span(*start_min, *end_min);
                let (os, oe) = span(*old_start, *old_end);
                days & old_days == *old_days && s <= os && e >= oe
            }
            _ => false,
        }
    }
}

impl Rule {
    /// このルールが、変更前の `old` を緩めていないか。緩めているときは、その理由を返す。
    ///
    /// 緩めない変更（本人もできる）: 名前の変更、対象の追加、制限の追加、制限の曜日の追加、
    /// 1 日の合計時間の短縮、使えない時間帯の拡大。無効のルールを有効にするのも厳しくする変更。
    pub fn check_not_looser_than(&self, old: &Rule) -> Result<(), String> {
        if old.enabled && !self.enabled {
            return Err("ルールを無効にするのは、友達だけができます".into());
        }
        // 対象は、減らせない（同じ対象は、識別子を減らさない限り名前の変更も可）
        for t in &old.targets {
            let kept = t
                .keys
                .iter()
                .all(|k| self.targets.iter().any(|n| n.keys.iter().any(|nk| nk == k)));
            if !kept {
                return Err("対象を減らすのは、友達だけができます".into());
            }
        }
        // 元の制限を、それぞれ別々の「同じか厳しい制限」が引き継いでいること
        let mut taken: Vec<Option<usize>> = vec![None; self.limits.len()];
        fn assign(i: usize, old: &[Limit], new: &[Limit], taken: &mut [Option<usize>], seen: &mut [bool]) -> bool {
            for j in 0..new.len() {
                if seen[j] || !new[j].is_at_least_as_strict_as(&old[i]) {
                    continue;
                }
                seen[j] = true;
                if taken[j].is_none_or(|k| assign(k, old, new, taken, seen)) {
                    taken[j] = Some(i);
                    return true;
                }
            }
            false
        }
        for i in 0..old.limits.len() {
            let mut seen = vec![false; self.limits.len()];
            if !assign(i, &old.limits, &self.limits, &mut taken, &mut seen) {
                return Err("制限を緩める変更は、友達だけができます（時間の延長・曜日や時間帯の縮小・制限の削除）".into());
            }
        }
        Ok(())
    }
}

/// `android:` `win:` `linux:` `domain:` のいずれかで始まり、空白と制御文字を含まない。
pub fn valid_key(k: &str) -> bool {
    let Some((prefix, rest)) = k.split_once(':') else {
        return false;
    };
    matches!(prefix, "android" | "win" | "linux" | "domain")
        && !rest.is_empty()
        && rest.len() <= 200
        && rest.chars().all(|c| !c.is_whitespace() && !c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_matches_subdomains() {
        assert!(key_matches("domain:example.com", "domain:example.com"));
        assert!(key_matches("domain:example.com", "domain:www.example.com"));
        assert!(!key_matches("domain:example.com", "domain:notexample.com"));
        assert!(!key_matches("android:a.b", "android:a.b.c"));
    }

    #[test]
    fn validation() {
        let mut r = Rule {
            id: "r".into(),
            name: "ゲーム".into(),
            targets: vec![Target {
                label: "A".into(),
                keys: vec!["android:com.a".into()],
            }],
            limits: vec![Limit::DailyTotal {
                days: ALL_DAYS,
                seconds: 3600,
            }],
            enabled: true,
        };
        assert!(r.validate().is_ok());
        r.limits = vec![Limit::Window {
            days: 0,
            start_min: 0,
            end_min: 60,
        }];
        assert!(r.validate().is_err());
        r.limits = vec![Limit::Window {
            days: 0x7f,
            start_min: 1320,
            end_min: 360,
        }];
        assert!(r.validate().is_ok());
        r.limits = vec![];
        assert!(r.validate().is_err());
        r.limits = vec![
            Limit::DailyTotal {
                days: ALL_DAYS,
                seconds: 3600,
            };
            MAX_LIMITS + 1
        ];
        assert!(r.validate().is_err());
        r.limits.truncate(2);
        assert!(r.validate().is_ok());
        r.limits.push(Limit::DailyTotal {
            days: 0,
            seconds: 60,
        });
        assert!(r.validate().is_err());
        r.limits.pop();
        r.targets[0].keys[0] = "bad key".into();
        assert!(r.validate().is_err());
    }

    #[test]
    fn name_is_optional_and_falls_back_to_the_targets() {
        let target = |label: &str, key: &str| Target {
            label: label.into(),
            keys: vec![key.into()],
        };
        let mut r = Rule {
            id: "r".into(),
            name: "".into(),
            targets: vec![target("YouTube", "android:com.youtube")],
            limits: vec![Limit::DailyTotal {
                days: ALL_DAYS,
                seconds: 3600,
            }],
            enabled: true,
        };
        assert!(r.validate().is_ok());
        assert_eq!(r.display_name(), "YouTube");
        r.targets.push(target("youtube.com", "domain:youtube.com"));
        r.targets.push(target("Twitch", "android:tv.twitch"));
        r.targets.push(target("Netflix", "android:com.netflix"));
        assert_eq!(r.display_name(), "YouTube ほか3件");
        r.name = "  動画  ".into();
        assert_eq!(r.display_name(), "動画");
    }

    #[test]
    fn overlapping_keys() {
        assert!(keys_overlap("android:a.b", "android:a.b"));
        assert!(keys_overlap("domain:youtube.com", "domain:m.youtube.com"));
        assert!(keys_overlap("domain:m.youtube.com", "domain:youtube.com"));
        assert!(!keys_overlap("domain:youtube.com", "domain:notyoutube.com"));
        assert!(!keys_overlap("android:a.b", "domain:a.b"));
    }

    fn rule(targets: &[&str], limits: Vec<Limit>) -> Rule {
        Rule {
            id: "r".into(),
            name: "ゲーム".into(),
            targets: targets
                .iter()
                .map(|k| Target {
                    label: (*k).into(),
                    keys: vec![format!("android:{k}")],
                })
                .collect(),
            limits,
            enabled: true,
        }
    }
    fn total(days: u8, seconds: u32) -> Limit {
        Limit::DailyTotal { days, seconds }
    }
    fn window(days: u8, start_min: u16, end_min: u16) -> Limit {
        Limit::Window { days, start_min, end_min }
    }

    #[test]
    fn changes_that_do_not_loosen_are_allowed() {
        let old = rule(&["com.a"], vec![total(0b0011111, 3600), window(ALL_DAYS, 1320, 360)]);
        let ok = |new: &Rule| assert_eq!(new.check_not_looser_than(&old), Ok(()), "{new:?}");
        // 名前
        let mut renamed = old.clone();
        renamed.name = "あとで直した".into();
        ok(&renamed);
        // 対象の追加
        ok(&rule(&["com.a", "com.b"], old.limits.clone()));
        // 制限の追加
        let mut more = old.limits.clone();
        more.push(total(ALL_DAYS, 60));
        ok(&rule(&["com.a"], more));
        // 曜日の追加
        ok(&rule(&["com.a"], vec![total(ALL_DAYS, 3600), window(ALL_DAYS, 1320, 360)]));
        // 1 日の合計時間の短縮
        ok(&rule(&["com.a"], vec![total(0b0011111, 1800), window(ALL_DAYS, 1320, 360)]));
        // 使えない時間の増加（前後に広げる。日をまたぐものも）
        ok(&rule(&["com.a"], vec![total(0b0011111, 3600), window(ALL_DAYS, 1260, 420)]));
        // 順番が変わってもよい
        ok(&rule(&["com.a"], vec![window(ALL_DAYS, 1320, 360), total(0b0011111, 3600)]));
        // 無効のルールを有効にする
        let mut off = old.clone();
        off.enabled = false;
        let mut on = old.clone();
        on.enabled = true;
        assert_eq!(on.check_not_looser_than(&off), Ok(()));
    }

    #[test]
    fn changes_that_loosen_are_refused() {
        let old = rule(&["com.a"], vec![total(ALL_DAYS, 3600), window(0b0011111, 1320, 360)]);
        let no = |new: &Rule| assert!(new.check_not_looser_than(&old).is_err(), "{new:?}");
        // 対象を減らす・入れ替える
        no(&rule(&["com.b"], old.limits.clone()));
        // 制限を消す
        no(&rule(&["com.a"], vec![total(ALL_DAYS, 3600)]));
        // 合計時間を延ばす
        no(&rule(&["com.a"], vec![total(ALL_DAYS, 7200), window(0b0011111, 1320, 360)]));
        // 曜日を減らす
        no(&rule(&["com.a"], vec![total(0b0011111, 3600), window(0b0011111, 1320, 360)]));
        // 時間帯を狭める
        no(&rule(&["com.a"], vec![total(ALL_DAYS, 3600), window(0b0011111, 1380, 360)]));
        no(&rule(&["com.a"], vec![total(ALL_DAYS, 3600), window(0b0011111, 1320, 300)]));
        // 1 つの制限で、元の 2 つを一度に引き継げない（種類をまたいでもだめ）
        no(&rule(&["com.a"], vec![total(ALL_DAYS, 60)]));
        // 無効にする
        let mut off = old.clone();
        off.enabled = false;
        no(&off);
        // 同じ制限を 2 つに分けて、片方を緩めるのもだめ
        let two = rule(&["com.a"], vec![total(ALL_DAYS, 3600), total(ALL_DAYS, 3600)]);
        assert!(rule(&["com.a"], vec![total(ALL_DAYS, 3600), total(ALL_DAYS, 7200)]).check_not_looser_than(&two).is_err());
    }

    #[test]
    fn daily_total_without_days_means_every_day() {
        // 曜日を持たない古い形（days なし）も読める
        let l: Limit = serde_json::from_str(r#"{"type":"daily_total","seconds":60}"#).unwrap();
        assert_eq!(
            l,
            Limit::DailyTotal {
                days: ALL_DAYS,
                seconds: 60
            }
        );
    }
}
