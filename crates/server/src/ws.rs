//! デバイスの WebSocket（`/api/ws/device`）。
//!
//! 認証は Tailscale のユーザー + ヘッダー `x-device-token`。
//! ハートビート（デバイスが 60 秒ごとに送る）が、生存の確認・状態の更新・方針の受け渡しを兼ねる。

use std::time::{Duration, Instant};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::HeaderMap;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use offsave_core::Ms;
use offsave_core::model::valid_key;
use offsave_core::protocol::{DeviceFlags, DeviceMsg, EventKind, ServerMsg, UNLOCK_MINUTES};
use tokio::sync::mpsc;

use crate::auth::CurrentUser;
use crate::db::now_ms;
use crate::error::{AppError, AppResult};
use crate::hub::Outgoing;
use crate::service::{build_policy, emit, recipients};
use crate::state::SharedState;
use crate::store::devices::{self, DeviceRow};
use crate::store::users::UserRow;
use crate::store::{events, usage};

pub const TOKEN_HEADER: &str = "x-device-token";
/// これだけ何も届かなければ、接続を閉じる（ハートビートは 60 秒ごと）。
const IDLE: Duration = Duration::from_secs(200);
const MAX_SEGMENTS: usize = 1000;

pub async fn device_ws(
    State(state): State<SharedState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> AppResult<Response> {
    let token = headers
        .get(TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("デバイスのトークンがありません"))?;
    let device = devices::find_by_token(&state.pool, token)
        .await?
        .ok_or_else(|| AppError::unauthorized("デバイスが登録されていません（削除されたかもしれません）"))?;
    if device.user_id != user.id {
        return Err(AppError::forbidden("このデバイスは別のユーザーのものです"));
    }
    Ok(ws
        .max_message_size(1 << 20)
        .on_upgrade(move |socket| run(state, user, device, socket)))
}

async fn run(state: SharedState, user: UserRow, device: DeviceRow, socket: WebSocket) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let conn_id = state.hub.register(&device.id, &user.id, tx);
    let (mut sink, mut stream) = socket.split();
    let mut last_rx = Instant::now();
    let mut catchup_done = false;

    loop {
        let deadline = tokio::time::Instant::from_std(last_rx + IDLE);
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                tracing::info!(device = %device.id, "ハートビートが途絶えたので接続を閉じます");
                break;
            }
            incoming = stream.next() => {
                last_rx = Instant::now();
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let replies = match serde_json::from_str::<DeviceMsg>(&text) {
                            Ok(msg) => handle(&state, &user, &device.id, msg, &mut catchup_done).await,
                            Err(e) => Err(AppError::bad_request(format!("メッセージを読めません: {e}"))),
                        };
                        match replies {
                            Ok(msgs) => {
                                for m in msgs {
                                    if send(&mut sink, &m).await.is_err() { break; }
                                }
                            }
                            Err(e) => {
                                let _ = send(&mut sink, &ServerMsg::Error { message: e.message }).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {} // ping / pong / binary
                }
            }
            out = rx.recv() => {
                let Some(out) = out else { break };
                let msg = match out {
                    Outgoing::Msg(m) => Some(m),
                    Outgoing::RefreshPolicy => build_policy(&state, &user_now(&state, &user).await, &device.id, now_ms())
                        .await
                        .ok()
                        .map(ServerMsg::Policy),
                };
                if let Some(m) = msg
                    && send(&mut sink, &m).await.is_err()
                {
                    break;
                }
            }
        }
    }
    state.hub.unregister(&device.id, conn_id);
}

/// 接続中にタイムゾーンが変わることがあるので、方針を作るたびに読み直す。
async fn user_now(state: &SharedState, user: &UserRow) -> UserRow {
    crate::store::users::get(&state.pool, &user.id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| user.clone())
}

async fn send(
    sink: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    msg: &ServerMsg,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).unwrap_or_default();
    sink.send(Message::Text(text.into())).await
}

async fn handle(
    state: &SharedState,
    user: &UserRow,
    device_id: &str,
    msg: DeviceMsg,
    catchup_done: &mut bool,
) -> AppResult<Vec<ServerMsg>> {
    let now = now_ms();
    match msg {
        DeviceMsg::Heartbeat {
            flags,
            foreground,
            last_event_id,
            ..
        } => {
            let user = user_now(state, user).await;
            let prev = devices::get(&state.pool, device_id)
                .await?
                .ok_or_else(|| AppError::not_found("デバイスがありません"))?;
            devices::heartbeat(&state.pool, device_id, now, &flags, foreground.as_deref()).await?;

            if prev.state == "suspect" {
                emit(state, &user.id, Some(device_id), EventKind::DeviceOnline,
                    &format!("{} が再び接続しました", prev.name)).await?;
            }
            if prev.state != "new" {
                let before = prev.flags();
                if before.protected() && !flags.protected() {
                    emit(state, &user.id, Some(device_id), EventKind::ProtectionDisabled,
                        &format!("{}: {}", prev.name, lost_protection(&before, &flags))).await?;
                } else if !before.protected() && flags.protected() {
                    emit(state, &user.id, Some(device_id), EventKind::ProtectionRestored,
                        &format!("{} の保護が有効になりました", prev.name)).await?;
                }
            }

            let mut out = Vec::new();
            if !*catchup_done {
                *catchup_done = true;
                if let Some(last) = last_event_id {
                    let to = recipients(state, &user.id).await?;
                    for event in events::list(&state.pool, &to, Some(&last), 100).await? {
                        out.push(ServerMsg::Event { event });
                    }
                }
            }
            out.push(ServerMsg::Policy(build_policy(state, &user, device_id, now).await?));
            Ok(out)
        }
        DeviceMsg::Usage { segments } => {
            if segments.len() > MAX_SEGMENTS {
                return Err(AppError::bad_request("区間が多すぎます"));
            }
            let valid: Vec<_> = segments
                .into_iter()
                .filter(|s| {
                    valid_key(&s.key)
                        && s.start < s.end
                        && s.end - s.start <= 86_400_000
                        && s.end <= now + 5 * 60_000
                        && s.start >= now - 48 * 3_600_000
                })
                .collect();
            usage::insert_segments(&state.pool, device_id, &user.id, &valid).await?;
            Ok(vec![])
        }
        DeviceMsg::Unlock {
            reason,
            started_at,
            until,
        } => {
            let reason = reason.trim().to_owned();
            if reason.is_empty() || reason.chars().count() > 500 {
                return Err(AppError::bad_request("理由は 1〜500 文字で書いてください"));
            }
            let max_len: Ms = state.config.unlock_minutes.max(UNLOCK_MINUTES) as Ms * 60_000 + 5_000;
            if until <= started_at
                || until - started_at > max_len
                || started_at > now + 5 * 60_000
                || started_at < now - 24 * 3_600_000
            {
                return Err(AppError::bad_request("一時解除の時刻が不正です"));
            }
            events::insert_unlock(&state.pool, &user.id, device_id, &reason, started_at, until).await?;
            emit(state, &user.id, Some(device_id), EventKind::TempUnlock, &reason).await?;
            state.hub.refresh_policy(&user.id);
            Ok(vec![])
        }
        DeviceMsg::Report { kind, message } => {
            if !matches!(
                kind,
                EventKind::TamperAttempt | EventKind::LimitReached | EventKind::ProtectionDisabled
            ) {
                return Err(AppError::bad_request("このイベントは送れません"));
            }
            let message: String = message.chars().take(300).collect();
            emit(state, &user.id, Some(device_id), kind, &message).await?;
            Ok(vec![])
        }
        DeviceMsg::Bye => {
            devices::set_state(&state.pool, device_id, "offline").await?;
            Ok(vec![])
        }
    }
}

fn lost_protection(before: &DeviceFlags, now: &DeviceFlags) -> String {
    let mut lost = Vec::new();
    if before.accessibility && !now.accessibility {
        lost.push("Accessibility が無効になりました");
    }
    if before.device_admin && !now.device_admin {
        lost.push("端末管理者が無効になりました");
    }
    lost.join("、")
}
