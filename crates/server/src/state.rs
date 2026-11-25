//! アプリケーション全体で共有する状態。

use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;
use crate::hub::Hub;
use crate::tailscale::Tailscale;

pub struct AppState {
    pub config: Config,
    pub pool: SqlitePool,
    pub hub: Hub,
    pub tailscale: Tailscale,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(config: Config, pool: SqlitePool) -> Self {
        let tailscale = Tailscale::new(config.tailscale_socket.clone());
        Self {
            config,
            pool,
            hub: Hub::default(),
            tailscale,
        }
    }
}
