//! 認証。接続元の Tailscale のユーザーを、このサービスのユーザーに対応付ける（disnans と同じ仕組み）。
//!
//! パスワードやトークンは使わない。Tailscale のネットワークの上で、接続元の IP アドレスから whois でユーザーを特定する。
//! デバイスの WebSocket だけは、さらにデバイスのトークンを要求する（`ws.rs`）。

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, FromRequestParts, Query, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::state::SharedState;
use crate::store::users::{self, UserRow};
use crate::tailscale::{Profile, default_display_name};

/// 開発モードでユーザーを指定するヘッダー。
pub const DEV_USER_HEADER: &str = "x-dev-user";
pub const DEV_DEFAULT_USER: &str = "dev@local";

/// 認証済みのユーザー。ハンドラーの引数で受け取る。
#[derive(Debug, Clone)]
pub struct CurrentUser(pub UserRow);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or_else(|| AppError::unauthorized("認証されていません"))
    }
}

#[derive(Deserialize)]
struct DevQuery {
    dev_user: Option<String>,
}

pub async fn middleware(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> AppResult<Response> {
    // CORS のプリフライトは認証しない
    if req.method() == axum::http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }
    let profile = if state.config.dev {
        dev_profile(&req)
    } else {
        state
            .tailscale
            .whois(addr)
            .await
            .map_err(AppError::internal)?
            .ok_or_else(|| AppError::unauthorized("Tailscale のユーザーを特定できません"))?
    };

    let user = users::get_or_create(
        &state.pool,
        &profile.login_name,
        &profile.display_name,
        &state.config.default_timezone,
    )
    .await?;
    req.extensions_mut().insert(CurrentUser(user));
    Ok(next.run(req).await)
}

/// 開発モード: `X-Dev-User` ヘッダー、`?dev_user=` クエリの順に見る。
fn dev_profile(req: &Request) -> Profile {
    let from_header = req
        .headers()
        .get(DEV_USER_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let from_query = || {
        Query::<DevQuery>::try_from_uri(req.uri())
            .ok()
            .and_then(|q| q.0.dev_user)
    };
    let login_name = from_header
        .or_else(from_query)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEV_DEFAULT_USER.to_owned());

    Profile {
        display_name: default_display_name(&login_name),
        avatar_url: None,
        login_name,
    }
}
