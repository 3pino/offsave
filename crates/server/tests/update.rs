//! サーバーの自動更新: 偽の GitHub（同じプロセスの axum）から取って、実行ファイルを置き換えるまで。

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::routing::get;
use offsave_server::update::{ASSET_NAME, Updater, sha256_hex};
use serde_json::json;

/// 偽の GitHub が返すもの。
#[derive(Clone)]
struct Fake {
    tag: String,
    prerelease: bool,
    binary: Arc<Vec<u8>>,
    sha_line: Arc<String>,
}

async fn start(fake: Fake) -> String {
    async fn release(State(f): State<Fake>, headers: axum::http::HeaderMap) -> axum::Json<serde_json::Value> {
        let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or_default();
        let base = format!("http://{host}");
        axum::Json(json!({
            "tag_name": f.tag,
            "draft": false,
            "prerelease": f.prerelease,
            "assets": [
                { "name": ASSET_NAME, "browser_download_url": format!("{base}/bin") },
                { "name": format!("{ASSET_NAME}.sha256"), "browser_download_url": format!("{base}/sha") },
            ],
        }))
    }
    async fn bin(State(f): State<Fake>) -> Vec<u8> {
        f.binary.to_vec()
    }
    async fn sha(State(f): State<Fake>) -> String {
        f.sha_line.to_string()
    }
    let app = Router::new()
        .route("/release", get(release))
        .route("/bin", get(bin))
        .route("/sha", get(sha))
        .with_state(fake);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/release")
}

/// `--version` に決まった文字列を返す実行ファイル（シェルスクリプト）。
fn script(version_output: &str) -> Vec<u8> {
    format!("#!/bin/sh\necho '{version_output}'\n").into_bytes()
}

fn write_exe(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake(tag: &str, binary: Vec<u8>) -> Fake {
    let sha_line = format!("{}  {ASSET_NAME}\n", sha256_hex(&binary));
    Fake {
        tag: tag.into(),
        prerelease: false,
        binary: Arc::new(binary),
        sha_line: Arc::new(sha_line),
    }
}

#[tokio::test]
async fn newer_release_replaces_the_binary_and_keeps_the_old_one() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("offsave-server");
    write_exe(&exe, &script("offsave-server 0.1.0"));
    let url = start(fake("v9.9.9", script("offsave-server 9.9.9"))).await;

    let updater = Updater::new(url, "0.1.0", exe.clone()).unwrap();
    assert_eq!(updater.check_once().await, Ok(Some("9.9.9".into())));

    assert!(std::fs::read_to_string(&exe).unwrap().contains("9.9.9"));
    assert!(std::fs::read_to_string(dir.path().join("offsave-server.prev")).unwrap().contains("0.1.0"));
    // 一時ファイルは残らない
    assert!(!dir.path().join("offsave-server.update").exists());
}

#[tokio::test]
async fn same_older_prerelease_and_unparsable_versions_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("offsave-server");
    let original = script("offsave-server 1.0.0");
    write_exe(&exe, &original);

    for (tag, pre) in [("v1.0.0", false), ("v0.9.0", false), ("v2.0.0", true), ("v2.0.0-rc.1", false), ("nightly", false)] {
        let mut f = fake(tag, script("offsave-server 2.0.0"));
        f.prerelease = pre;
        let updater = Updater::new(start(f).await, "1.0.0", exe.clone()).unwrap();
        assert_eq!(updater.check_once().await, Ok(None), "{tag}");
    }
    assert_eq!(std::fs::read(&exe).unwrap(), original);
}

#[tokio::test]
async fn bad_downloads_never_replace_the_binary() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("offsave-server");
    let original = script("offsave-server 1.0.0");
    write_exe(&exe, &original);

    // sha256 が合わない
    let mut f = fake("v2.0.0", script("offsave-server 2.0.0"));
    f.sha_line = Arc::new(format!("{}  {ASSET_NAME}\n", sha256_hex(b"other")));
    let updater = Updater::new(start(f).await, "1.0.0", exe.clone()).unwrap();
    assert!(updater.check_once().await.unwrap_err().contains("sha256"));

    // 版を名乗らない（別の版を返す）バイナリ
    let updater = Updater::new(start(fake("v2.0.0", script("offsave-server 3.0.0"))).await, "1.0.0", exe.clone()).unwrap();
    assert!(updater.check_once().await.unwrap_err().contains("--version"));

    assert_eq!(std::fs::read(&exe).unwrap(), original);
    assert!(!dir.path().join("offsave-server.update").exists());
}

#[tokio::test]
async fn gives_up_on_a_version_after_repeated_failures() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("offsave-server");
    write_exe(&exe, &script("offsave-server 1.0.0"));
    let updater = Updater::new(start(fake("v2.0.0", script("offsave-server 3.0.0"))).await, "1.0.0", exe).unwrap();
    for _ in 0..offsave_server::update::MAX_FAILURES {
        assert!(updater.check_once().await.is_err());
    }
    // 諦めたあとは、同じ版を試さない
    assert_eq!(updater.check_once().await, Ok(None));
}
