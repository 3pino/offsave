use std::net::SocketAddr;
use std::sync::Arc;

use offsave_server::config::Config;
use offsave_server::state::AppState;
use offsave_server::{app, db, watchdog};

/// 再起動を要求されてから、接続が閉じきらなくても終わるまでの時間。
const RESTART_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

#[tokio::main]
async fn main() {
    // 自動更新が、入れ替える前に新しいバイナリの版を確かめるのに使う
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("{}", offsave_server::update::version_line(env!("CARGO_PKG_VERSION")));
        return;
    }

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

    // 自動更新は、設定したときだけ（既定は無効。取得先は本番の固定値）
    let auto_update = std::env::var("OFFSAVE_AUTO_UPDATE").is_ok_and(|v| v == "1");
    tracing::info!(
        "offsave-server {}（自動更新: {}）",
        env!("CARGO_PKG_VERSION"),
        if auto_update { "有効" } else { "無効" }
    );
    if auto_update {
        offsave_server::update::spawn_update_task(state.clone(), offsave_server::update::RELEASE_API_URL.into());
    }

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{bind} で待ち受けられません: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!("offsave-server を {bind} で起動しました");
    let restart = state.clone();
    let serve = axum::serve(
        listener,
        app(state.clone()).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move { restart.restart.wait().await });
    // 自動更新で入れ替えたあとは、接続が閉じきらなくても一定の時間で終える
    let forced = async {
        state.restart.wait().await;
        tokio::time::sleep(RESTART_GRACE).await;
    };
    tokio::select! {
        result = serve => {
            if let Err(e) = result {
                eprintln!("サーバーが止まりました: {e}");
                std::process::exit(1);
            }
        }
        _ = forced => tracing::warn!("接続が閉じきらないので、そのまま終了します"),
    }
    if state.restart.requested() {
        // systemd（Restart=on-failure）が、新しいバイナリで起こし直す
        std::process::exit(offsave_server::update::RESTART_EXIT_CODE);
    }
}
