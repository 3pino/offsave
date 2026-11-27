//! アプリケーション全体で共有する状態。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use sqlx::SqlitePool;

use crate::config::Config;
use crate::hub::Hub;
use crate::tailscale::Tailscale;
use crate::update::Restart;

pub struct AppState {
    pub config: Config,
    pub pool: SqlitePool,
    pub hub: Hub,
    pub tailscale: Tailscale,
    /// 自動更新で入れ替えたあとの、再起動の要求。
    pub restart: Restart,
    /// 保護が外れたが、まだ友達に知らせていない（猛予中の）デバイス。
    pub pending_loss: Mutex<HashSet<String>>,
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
            restart: Restart::default(),
            pending_loss: Mutex::default(),
        }
    }
}
