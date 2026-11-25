//! REST と WebSocket が共有する処理: イベントの発行、デバイスに渡す方針、画面に出す状態。

use std::collections::BTreeMap;

use offsave_core::Ms;
use offsave_core::engine::used_ms;
use offsave_core::intervals::{Segment, merge, union_len};
use offsave_core::protocol::{
    Event, EventKind, Policy, RuleStatus, UsageItem, UserStatus,
};
use offsave_core::time::logical_day;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store::users::UserRow;
use crate::store::{devices, events, friends, rules, usage, users};
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
        rules: stored.into_iter().map(|s| s.rule).collect(),
        overrides: overrides.iter().map(|o| o.to_override()).collect(),
        unlock_until,
        last_unlock_end,
        unlock_minutes: state.config.unlock_minutes,
        others: merge_by_key(others),
    })
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
    })
}
