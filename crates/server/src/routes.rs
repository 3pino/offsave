//! REST の API（`/api`）。

use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use offsave_core::model::{Limit, Rule, Target};
use offsave_core::protocol::{
    DeviceInfo, Event, EventKind, Friend, FriendState, User, UserSettings, UserStatus,
};
use offsave_core::time::{logical_day, parse_date};
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::db::now_ms;
use crate::error::{AppError, AppResult};
use crate::service::{self, can_view, clock_of, emit, recipients};
use crate::state::SharedState;
use crate::store::friends::RequestResult;
use crate::store::users::UserRow;
use crate::store::{devices, events, friends, rules, users};
use crate::timeutil::valid_timezone;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/me", get(me))
        .route("/me/settings", put(update_settings))
        .route("/friends", get(list_friends).post(request_friend))
        .route("/friends/{id}/accept", post(accept_friend))
        .route("/friends/{id}", delete(remove_friend))
        .route("/devices", post(register_device))
        .route("/devices/{id}", delete(remove_device))
        .route("/users/{id}/status", get(user_status))
        .route("/users/{id}/rules", post(create_rule))
        .route("/users/{id}/overrides", post(create_override))
        .route("/rules/{id}", put(update_rule).delete(delete_rule))
        .route("/overrides/{id}", delete(delete_override))
        .route("/events", get(list_events))
        .route("/ws/device", get(crate::ws::device_ws))
}

#[derive(Serialize)]
struct MeResponse {
    user: User,
    settings: UserSettings,
}

async fn me(CurrentUser(user): CurrentUser) -> Json<MeResponse> {
    Json(MeResponse {
        user: user.to_user(),
        settings: user.settings(),
    })
}

async fn update_settings(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<UserSettings>,
) -> AppResult<Json<UserSettings>> {
    if !valid_timezone(&body.timezone) {
        return Err(AppError::bad_request("タイムゾーンが不正です（例: Asia/Tokyo）"));
    }
    if body.day_boundary_min >= 1440 {
        return Err(AppError::bad_request("日付の境界は 0〜1439 分で指定してください"));
    }
    users::update_settings(&state.pool, &user.id, &body.timezone, body.day_boundary_min).await?;
    emit(
        &state,
        &user.id,
        None,
        EventKind::SettingsChanged,
        &format!(
            "タイムゾーンを {}、日付の境界を {:02}:{:02} にしました",
            body.timezone,
            body.day_boundary_min / 60,
            body.day_boundary_min % 60
        ),
    )
    .await?;
    state.hub.refresh_policy(&user.id);
    Ok(Json(body))
}

// ---- 友達 ----

async fn list_friends(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<Friend>>> {
    let list = friends::list(&state.pool, &user.id).await?;
    Ok(Json(
        list.into_iter()
            .map(|(u, state)| Friend {
                user: u.to_user(),
                state,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct FriendRequest {
    login_name: String,
}

async fn request_friend(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<FriendRequest>,
) -> AppResult<Json<Friend>> {
    let other = users::by_login(&state.pool, body.login_name.trim())
        .await?
        .ok_or_else(|| AppError::not_found("そのユーザーはまだ offsave を使っていません（先に相手がアプリを開く必要があります）"))?;
    if other.id == user.id {
        return Err(AppError::bad_request("自分は友達にできません"));
    }
    let (result_state, event_kind) = match friends::request(&state.pool, &user.id, &other.id).await? {
        RequestResult::Requested => (FriendState::PendingOut, None),
        RequestResult::Accepted => (FriendState::Accepted, Some(EventKind::FriendAdded)),
        RequestResult::AlreadyFriends => return Err(AppError::conflict("すでに友達です")),
        RequestResult::AlreadyRequested => return Err(AppError::conflict("すでに申請しています")),
    };
    if let Some(kind) = event_kind {
        let msg = format!("{} と {} が友達になりました", user.display_name, other.display_name);
        emit(&state, &user.id, None, kind, &msg).await?;
        emit(&state, &other.id, None, kind, &msg).await?;
        state.hub.refresh_policy(&user.id);
        state.hub.refresh_policy(&other.id);
    }
    Ok(Json(Friend {
        user: other.to_user(),
        state: result_state,
    }))
}

async fn accept_friend(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(other_id): Path<String>,
) -> AppResult<Json<Friend>> {
    let other = users::get(&state.pool, &other_id)
        .await?
        .ok_or_else(|| AppError::not_found("ユーザーがいません"))?;
    if !friends::accept(&state.pool, &user.id, &other.id).await? {
        return Err(AppError::not_found("承認できる申請がありません"));
    }
    let msg = format!("{} と {} が友達になりました", user.display_name, other.display_name);
    emit(&state, &user.id, None, EventKind::FriendAdded, &msg).await?;
    emit(&state, &other.id, None, EventKind::FriendAdded, &msg).await?;
    Ok(Json(Friend {
        user: other.to_user(),
        state: FriendState::Accepted,
    }))
}

/// 申請の取り消し・拒否・友達の解消（一方的にできる）。
async fn remove_friend(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(other_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let other = users::get(&state.pool, &other_id)
        .await?
        .ok_or_else(|| AppError::not_found("ユーザーがいません"))?;
    // 解消の前に、イベントを見られる人を決める（解消後は友達ではなくなる）
    let was_accepted = match friends::remove(&state.pool, &user.id, &other.id).await? {
        Some(accepted) => accepted,
        None => return Err(AppError::not_found("関係がありません")),
    };
    if was_accepted {
        let msg = format!("{} が {} との友達を解消しました", user.display_name, other.display_name);
        // 解消した本人と相手の両方に残す（解消後なので、それぞれ自分宛てに記録する）
        let e1 = events::insert(&state.pool, &user.id, None, EventKind::FriendRemoved, &msg).await?;
        let e2 = events::insert(&state.pool, &other.id, None, EventKind::FriendRemoved, &msg).await?;
        state.hub.push_event(&e1, &[user.id.clone(), other.id.clone()]);
        state.hub.push_event(&e2, &[user.id.clone(), other.id.clone()]);
        state.hub.refresh_policy(&user.id);
        state.hub.refresh_policy(&other.id);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- デバイス ----

#[derive(Deserialize)]
struct DeviceRegistration {
    name: String,
    platform: String,
}

#[derive(Serialize)]
struct DeviceRegistered {
    device: DeviceInfo,
    /// この 1 回しか見られない。デバイスに保存する。
    token: String,
}

async fn register_device(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<DeviceRegistration>,
) -> AppResult<Json<DeviceRegistered>> {
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::bad_request("デバイス名は 1〜60 文字にしてください"));
    }
    if !matches!(body.platform.as_str(), "android" | "windows" | "linux") {
        return Err(AppError::bad_request("platform は android / windows / linux のいずれかです"));
    }
    let (row, token) = devices::create(&state.pool, &user.id, name, &body.platform).await?;
    emit(
        &state,
        &user.id,
        Some(&row.id),
        EventKind::DeviceAdded,
        &format!("デバイス「{name}」（{}）が追加されました", body.platform),
    )
    .await?;
    Ok(Json(DeviceRegistered {
        device: row.to_info(),
        token,
    }))
}

async fn remove_device(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let device = devices::get(&state.pool, &id)
        .await?
        .filter(|d| d.user_id == user.id && d.removed_at.is_none())
        .ok_or_else(|| AppError::not_found("デバイスがありません"))?;
    devices::remove(&state.pool, &device.id).await?;
    emit(
        &state,
        &user.id,
        Some(&device.id),
        EventKind::DeviceRemoved,
        &format!("デバイス「{}」が削除されました", device.name),
    )
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- 状態・ルール ----

async fn target_user(state: &SharedState, id: &str) -> AppResult<UserRow> {
    users::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("ユーザーがいません"))
}

async fn user_status(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<UserStatus>> {
    if !can_view(&state, &me.id, &id).await? {
        return Err(AppError::forbidden("友達だけが見られます"));
    }
    let target = target_user(&state, &id).await?;
    Ok(Json(service::user_status(&state, &target, now_ms()).await?))
}

#[derive(Deserialize)]
struct RuleInput {
    name: String,
    targets: Vec<Target>,
    limit: Limit,
    #[serde(default = "yes")]
    enabled: bool,
}

fn yes() -> bool {
    true
}

impl RuleInput {
    fn into_rule(self) -> AppResult<Rule> {
        let rule = Rule {
            id: String::new(),
            name: self.name.trim().to_owned(),
            targets: self.targets,
            limit: self.limit,
            enabled: self.enabled,
        };
        rule.validate().map_err(AppError::bad_request)?;
        Ok(rule)
    }
}

/// 本人か友達か。`friend_only` なら友達だけ（本人は不可）。
async fn require_access(
    state: &SharedState,
    me: &UserRow,
    owner_id: &str,
    friend_only: bool,
) -> AppResult<()> {
    if me.id == owner_id {
        return if friend_only {
            Err(AppError::forbidden("自分では変えられません（友達に頼んでください）"))
        } else {
            Ok(())
        };
    }
    if crate::store::friends::is_friend(&state.pool, &me.id, owner_id).await? {
        Ok(())
    } else {
        Err(AppError::forbidden("友達だけが操作できます"))
    }
}

async fn rule_changed(state: &SharedState, owner: &UserRow, by: &UserRow, what: &str) -> AppResult<()> {
    emit(
        state,
        &owner.id,
        None,
        EventKind::RuleChanged,
        &format!("{} が {what}", by.display_name),
    )
    .await?;
    state.hub.refresh_policy(&owner.id);
    Ok(())
}

/// ルールを足す。自分に足せるのは「厳しくする」ためなので、本人でもよい。
async fn create_rule(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Path(owner_id): Path<String>,
    Json(body): Json<RuleInput>,
) -> AppResult<Json<Rule>> {
    require_access(&state, &me, &owner_id, false).await?;
    let owner = target_user(&state, &owner_id).await?;
    let mut rule = body.into_rule()?;
    rule.id = rules::insert(&state.pool, &owner.id, &me.id, &rule).await?;
    rule_changed(&state, &owner, &me, &format!("ルール「{}」を追加しました", rule.name)).await?;
    Ok(Json(rule))
}

async fn update_rule(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<String>,
    Json(body): Json<RuleInput>,
) -> AppResult<Json<Rule>> {
    let stored = rules::get(&state.pool, &id)
        .await?
        .ok_or_else(|| AppError::not_found("ルールがありません"))?;
    require_access(&state, &me, &stored.owner_id, true).await?;
    let owner = target_user(&state, &stored.owner_id).await?;
    let mut rule = body.into_rule()?;
    rule.id = id.clone();
    rules::update(&state.pool, &id, &rule).await?;
    rule_changed(&state, &owner, &me, &format!("ルール「{}」を変更しました", rule.name)).await?;
    Ok(Json(rule))
}

async fn delete_rule(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let stored = rules::get(&state.pool, &id)
        .await?
        .ok_or_else(|| AppError::not_found("ルールがありません"))?;
    require_access(&state, &me, &stored.owner_id, true).await?;
    let owner = target_user(&state, &stored.owner_id).await?;
    rules::delete(&state.pool, &id).await?;
    rule_changed(&state, &owner, &me, &format!("ルール「{}」を削除しました", stored.rule.name)).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
struct OverrideInput {
    /// 論理日（`YYYY-MM-DD`）。
    date: String,
    /// 省略すると全ルール。
    rule_id: Option<String>,
}

async fn create_override(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Path(owner_id): Path<String>,
    Json(body): Json<OverrideInput>,
) -> AppResult<Json<offsave_core::model::Override>> {
    require_access(&state, &me, &owner_id, true).await?;
    let owner = target_user(&state, &owner_id).await?;
    if parse_date(&body.date).is_none() {
        return Err(AppError::bad_request("日付は YYYY-MM-DD で指定してください"));
    }
    let today = logical_day(now_ms(), &clock_of(&owner, now_ms())).date;
    if body.date < today {
        return Err(AppError::bad_request("過去の日付は指定できません"));
    }
    if let Some(rule_id) = &body.rule_id {
        let stored = rules::get(&state.pool, rule_id).await?;
        if stored.is_none_or(|s| s.owner_id != owner.id) {
            return Err(AppError::bad_request("そのルールはありません"));
        }
    }
    let id = rules::insert_override(&state.pool, &owner.id, &me.id, &body.date, body.rule_id.as_deref()).await?;
    rule_changed(&state, &owner, &me, &format!("{} の制限を外しました", body.date)).await?;
    Ok(Json(offsave_core::model::Override {
        id,
        date: body.date,
        rule_id: body.rule_id,
    }))
}

async fn delete_override(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let row = rules::get_override(&state.pool, &id)
        .await?
        .ok_or_else(|| AppError::not_found("オーバーライドがありません"))?;
    require_access(&state, &me, &row.owner_id, true).await?;
    let owner = target_user(&state, &row.owner_id).await?;
    rules::delete_override(&state.pool, &id).await?;
    rule_changed(&state, &owner, &me, &format!("{} の制限の解除を取り消しました", row.date)).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- イベント ----

#[derive(Deserialize)]
struct EventsQuery {
    limit: Option<i64>,
}

/// 自分と友達についてのイベント（新しい順）。
async fn list_events(
    State(state): State<SharedState>,
    CurrentUser(me): CurrentUser,
    Query(q): Query<EventsQuery>,
) -> AppResult<Json<Vec<Event>>> {
    let to = recipients(&state, &me.id).await?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    Ok(Json(events::list(&state.pool, &to, None, limit).await?))
}
