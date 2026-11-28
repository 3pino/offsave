//! REST と WebSocket が共有する処理: イベントの発行、デバイスに渡す方針、画面に出す状態。

use std::collections::BTreeMap;

use offsave_core::Ms;
use offsave_core::engine::used_ms;
use offsave_core::intervals::{Segment, merge, union_len};
use offsave_core::protocol::{
    Event, EventKind, Policy, RuleStatus, UsageBucket, UsageHistory, UsageItem, UserStatus,
};
use offsave_core::time::{HOUR_MS, logical_day, logical_day_of};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store::users::UserRow;
use crate::store::{devices, events, friends, pause_apps, rules, usage, users};
use crate::timeutil::clock_for;

/// 自分か、承認済みの友達か。
pub async fn can_view(state: &AppState, caller: &str, target: &str) -> AppResult<bool> {
    Ok(caller == target || friends::is_friend(&state.pool, caller, target).await?)
}

/// イベントを見られる人（本人と承認済みの友達）。
pub async fn recipients(state: &AppState, user_id: &str) -> AppResult<Vec<String>> {
    let mut ids = friends::friend_ids(&state.pool, user_id).await?;
    ids.push(user_id.to_owned());
    Ok(ids)
}

/// イベントを記録し、接続中の本人と友達のデバイスに送る。
pub async fn emit(
    state: &AppState,
    user_id: &str,
    device_id: Option<&str>,
    kind: EventKind,
    message: &str,
) -> AppResult<Event> {
    let event = events::insert(&state.pool, user_id, device_id, kind, message).await?;
    let to = recipients(state, user_id).await?;
    state.hub.push_event(&event, &to);
    Ok(event)
}

/// 「今」のユーザーの時計。
pub fn clock_of(user: &UserRow, now: Ms) -> offsave_core::time::Clock {
    clock_for(&user.timezone, user.day_boundary_min as u32, now)
}

/// 識別子ごとに区間をまとめる。
fn merge_by_key(segments: Vec<Segment>) -> Vec<Segment> {
    let mut by_key: BTreeMap<String, Vec<(Ms, Ms)>> = BTreeMap::new();
    for s in segments {
        by_key.entry(s.key).or_default().push((s.start, s.end));
    }
    by_key
        .into_iter()
        .flat_map(|(key, ranges)| {
            merge(ranges)
                .into_iter()
                .map(move |(start, end)| Segment {
                    key: key.clone(),
                    start,
                    end,
                })
        })
        .collect()
}

pub async fn build_policy(
    state: &AppState,
    user: &UserRow,
    device_id: &str,
    now: Ms,
) -> AppResult<Policy> {
    let clock = clock_of(user, now);
    let day = logical_day(now, &clock);
    let stored = rules::list(&state.pool, &user.id).await?;
    let overrides = rules::list_overrides(&state.pool, &user.id, &day.date).await?;
    let (unlock_until, last_unlock_end) = events::unlock_state(&state.pool, &user.id, now).await?;
    let others = usage::segments_of_user(&state.pool, &user.id, Some(device_id), day.start, day.end).await?;
    Ok(Policy {
        server_time: now,
        clock,
        // 名前が空のルールは、対象から作った名前を入れて渡す（古いアプリでも、制限の画面に名前が出る）
        rules: stored
            .into_iter()
            .map(|s| {
                let mut rule = s.rule;
                rule.name = rule.display_name();
                rule
            })
            .collect(),
        overrides: overrides.iter().map(|o| o.to_override()).collect(),
        unlock_until,
        last_unlock_end,
        unlock_minutes: state.config.unlock_minutes,
        others: merge_by_key(others),
        pause_apps: pause_apps::list(&state.pool, &user.id).await?,
    })
}

/// 日ごと（`hourly` なら `from` の 1 日を 1 時間ごと）の利用。`from` は論理日の日付（`days` 日ぶん）。
pub async fn usage_history(
    state: &AppState,
    user: &UserRow,
    from_day: i64,
    days: u32,
    hourly: bool,
    now: Ms,
) -> AppResult<UsageHistory> {
    let clock = clock_of(user, now);
    let days = if hourly { 1 } else { days.max(1) };
    let day_list: Vec<_> = (0..days as i64).map(|i| logical_day_of(from_day + i, &clock)).collect();
    let lo = day_list.first().map_or(0, |d| d.start);
    let hi = day_list.last().map_or(0, |d| d.end);
    let segments = usage::segments_of_user(&state.pool, &user.id, None, lo, hi).await?;
    let mut by_key: BTreeMap<&str, Vec<(Ms, Ms)>> = BTreeMap::new();
    for s in &segments {
        by_key.entry(&s.key).or_default().push((s.start, s.end));
    }
    let items_in = |from: Ms, to: Ms| -> Vec<UsageItem> {
        let mut items: Vec<UsageItem> = by_key
            .iter()
            .filter_map(|(key, ranges)| {
                let seconds = (union_len(ranges.iter().copied(), from, to) / 1000) as u32;
                (seconds > 0).then(|| UsageItem {
                    key: (*key).to_owned(),
                    seconds,
                })
            })
            .collect();
        items.sort_by(|a, b| b.seconds.cmp(&a.seconds).then_with(|| a.key.cmp(&b.key)));
        items
    };
    let mut buckets = Vec::new();
    for day in &day_list {
        if hourly {
            for h in 0..24 {
                let (start, end) = (day.start + h * HOUR_MS, day.start + (h + 1) * HOUR_MS);
                buckets.push(UsageBucket {
                    start,
                    end,
                    date: day.date.clone(),
                    weekday: day.weekday,
                    items: items_in(start, end),
                });
            }
        } else {
            buckets.push(UsageBucket {
                start: day.start,
                end: day.end,
                date: day.date.clone(),
                weekday: day.weekday,
                items: items_in(day.start, day.end),
            });
        }
    }
    Ok(UsageHistory { hourly, buckets })
}

pub async fn user_status(state: &AppState, user: &UserRow, now: Ms) -> AppResult<UserStatus> {
    let clock = clock_of(user, now);
    let day = logical_day(now, &clock);
    let stored = rules::list(&state.pool, &user.id).await?;
    let overrides = rules::list_overrides(&state.pool, &user.id, &day.date).await?;
    let (unlock_until, _) = events::unlock_state(&state.pool, &user.id, now).await?;
    let segments = usage::segments_of_user(&state.pool, &user.id, None, day.start, day.end).await?;

    let mut rule_statuses = Vec::with_capacity(stored.len());
    for s in &stored {
        let author = users::get(&state.pool, &s.author_id)
            .await?
            .ok_or_else(|| AppError::internal("ルールの作成者がいません"))?;
        rule_statuses.push(RuleStatus {
            rule: s.rule.clone(),
            author: author.to_user(),
            used_seconds: (used_ms(&s.rule, &segments, day.start, day.end) / 1000) as u32,
        });
    }

    let mut by_key: BTreeMap<String, Vec<(Ms, Ms)>> = BTreeMap::new();
    for s in &segments {
        by_key.entry(s.key.clone()).or_default().push((s.start, s.end));
    }
    let mut usage_today: Vec<UsageItem> = by_key
        .into_iter()
        .map(|(key, ranges)| UsageItem {
            key,
            seconds: (union_len(ranges, day.start, day.end) / 1000) as u32,
        })
        .collect();
    usage_today.sort_by(|a, b| b.seconds.cmp(&a.seconds).then_with(|| a.key.cmp(&b.key)));

    let devices = devices::list_for_user(&state.pool, &user.id).await?;
    Ok(UserStatus {
        user: user.to_user(),
        settings: user.settings(),
        devices: devices.iter().map(|d| d.to_info()).collect(),
        rules: rule_statuses,
        overrides: overrides.iter().map(|o| o.to_override()).collect(),
        unlock_until,
        date: day.date,
        usage_today,
        labels: usage::labels_of_user(&state.pool, &user.id).await?,
        pause_apps: pause_apps::list(&state.pool, &user.id).await?,
    })
}
