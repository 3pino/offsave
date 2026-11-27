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

impl Rule {
    pub fn matches(&self, key: &str) -> bool {
        self.targets
            .iter()
            .any(|t| t.keys.iter().any(|k| key_matches(k, key)))
    }

    /// 保存してよい形か。だめなら理由を返す。
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.chars().count() > 60 {
            return Err("ルール名は 1〜60 文字にしてください".into());
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
