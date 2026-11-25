//! offsave のサーバー（axum + SQLite）。Tailscale の中だけで動かす。

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod hub;
pub mod routes;
pub mod service;
pub mod state;
pub mod store;
pub mod tailscale;
pub mod timeutil;
pub mod watchdog;
pub mod ws;

#[cfg(feature = "test-support")]
pub mod testing;

use axum::Router;
use axum::middleware;
use tower_http::cors::CorsLayer;

use crate::state::SharedState;

/// `/api` を持つルーター。`into_make_service_with_connect_info::<SocketAddr>()` で動かす。
pub fn app(state: SharedState) -> Router {
    let api = routes::router()
        .layer(middleware::from_fn_with_state(state.clone(), auth::middleware))
        .layer(CorsLayer::very_permissive());
    Router::new().nest("/api", api).with_state(state)
}
