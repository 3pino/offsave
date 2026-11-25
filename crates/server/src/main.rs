use std::net::SocketAddr;
use std::sync::Arc;

use offsave_server::config::Config;
use offsave_server::state::AppState;
use offsave_server::{app, db, watchdog};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RUST_LOG")
                .unwrap_or_else(|_| "info,offsave_server=debug".into()),
        )
        .init();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("設定が不正です: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
        eprintln!("データのディレクトリを作れません（{}）: {e}", config.data_dir.display());
        std::process::exit(1);
    }
    let pool = match db::connect(&config.db_path()).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("DB を開けません: {e}");
            std::process::exit(1);
        }
    };

    let bind = config.bind;
    if config.dev {
        tracing::warn!("開発モードです。認証をしません（X-Dev-User で誰にでもなれます）");
    }
    let state = Arc::new(AppState::new(config, pool));
    watchdog::spawn(state.clone());

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{bind} で待ち受けられません: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!("offsave-server を {bind} で起動しました");
    if let Err(e) = axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        eprintln!("サーバーが止まりました: {e}");
        std::process::exit(1);
    }
}
