//! 結合テスト用の開発モードのサーバー。`test-support` のときだけ入る。

use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::Config;
use crate::state::AppState;
use crate::{app, db};

pub struct DevServer {
    pub addr: SocketAddr,
    pub state: Arc<AppState>,
    _dir: tempfile::TempDir,
}

impl DevServer {
    pub async fn start() -> Self {
        Self::start_with(|_| {}).await
    }

    pub async fn start_with(tweak: impl FnOnce(&mut Config)) -> Self {
        let dir = tempfile::tempdir().expect("一時ディレクトリ");
        let mut config = Config {
            data_dir: dir.path().to_path_buf(),
            bind: "127.0.0.1:0".parse().unwrap(),
            dev: true,
            tailscale_socket: "/nonexistent".into(),
            default_timezone: "Asia/Tokyo".into(),
            suspect_after_ms: 180_000,
            protection_grace_ms: 100,
            unlock_minutes: offsave_core::protocol::UNLOCK_MINUTES,
        };
        tweak(&mut config);
        let pool = db::connect(&config.db_path()).await.expect("DB");
        let state = Arc::new(AppState::new(config, pool));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = app(state.clone());
        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        Self {
            addr,
            state,
            _dir: dir,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub fn ws_url(&self, path: &str, dev_user: &str) -> String {
        format!("ws://{}{}?dev_user={}", self.addr, path, dev_user)
    }
}
