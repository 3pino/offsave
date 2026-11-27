//! サーバーの自動更新（`OFFSAVE_AUTO_UPDATE=1` のときだけ）。
//!
//! GitHub の最新リリース（正式版）を定期的に見て、自分より新しい版があれば資産を取り、sha256 を照合し、
//! `--version` で想定の版を返すことを確かめてから、今のバイナリと置き換える。
//! 置き換える前に DB のバックアップを取る。置き換えたあとは終了コード [`RESTART_EXIT_CODE`] で終わり、
//! systemd（`Restart=on-failure`）に起動し直してもらう。
//! 失敗は警告のログだけで、次の周期に再試行する。同じ版で [`MAX_FAILURES`] 回失敗したら、その版は再起動まで諦める。

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::Notify;

use crate::state::SharedState;

/// 本番の最新リリースの取得先（GitHub API）。環境変数では差し替えない（テストはコードから渡す）。
pub const RELEASE_API_URL: &str = "https://api.github.com/repos/3pino/offsave/releases/latest";
/// Linux x86_64 の資産名。sha256 は `<資産名>.sha256`。
pub const ASSET_NAME: &str = "offsave-server_linux-x64";
/// 置き換えたあとの終了コード（EX_TEMPFAIL）。systemd の `Restart=on-failure` が非 0 の終了で再起動する。
pub const RESTART_EXIT_CODE: i32 = 75;
/// 同じ版での失敗の上限。これを超えたら、その版は再起動まで試さない。
pub const MAX_FAILURES: u32 = 3;
/// 最初の確認までの待ち時間（起動直後の混雑を避ける）。
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(60);
/// 確認の間隔（GitHub の未認証の上限 60 回/時に対して余裕のある 6 回/時）。
pub const CHECK_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// 新しいバイナリの上限（これを超えたら取り込まない）。
const MAX_BINARY_BYTES: u64 = 200 * 1024 * 1024;
const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
/// 残しておく DB のバックアップの数。
const KEEP_BACKUPS: usize = 5;

/// 再起動の要求。置き換えたあと、サーバーを穏やかに止めるために使う。
#[derive(Default)]
pub struct Restart {
    requested: AtomicBool,
    notify: Notify,
}

impl Restart {
    pub fn request(&self) {
        self.requested.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }

    /// 再起動が要求されるまで待つ。
    pub async fn wait(&self) {
        loop {
            // 先に作ってから確かめることで、要求との競合で取りこぼさない
            let notified = self.notify.notified();
            if self.requested() {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// sha256sum の出力（`<hex>  <name>` や `<hex>`）から、先頭の 64 桁の 16 進数を小文字で取り出す。
pub fn parse_sha256_line(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    (token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| token.to_ascii_lowercase())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// リリースから、取る資産（バイナリと sha256）を選ぶ。
fn select_assets(release: &Release) -> Option<(&Asset, &Asset)> {
    let bin = release.assets.iter().find(|a| a.name == ASSET_NAME)?;
    let sha_name = format!("{ASSET_NAME}.sha256");
    let sha = release.assets.iter().find(|a| a.name == sha_name)?;
    Some((bin, sha))
}

/// `--version` が返すべき文字列（`offsave-server 1.2.3`）。
pub fn version_line(version: &str) -> String {
    format!("offsave-server {version}")
}

/// 入れ替えの直前の処理。
pub type BeforeSwap = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// 自動更新の本体。HTTP の取得先と、置き換える実行ファイルを持つ（テストでは両方を差し替える）。
pub struct Updater {
    client: reqwest::Client,
    api_url: String,
    current: String,
    exe: PathBuf,
    /// 同じ版での失敗回数
    failures: Mutex<HashMap<String, u32>>,
    before_swap: Option<BeforeSwap>,
}

impl Updater {
    pub fn new(api_url: String, current: &str, exe: PathBuf) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("offsave-server/{current}"))
            .timeout(DOWNLOAD_TIMEOUT)
            .build()
            .map_err(|e| format!("HTTP クライアントを作れません: {e}"))?;
        Ok(Self {
            client,
            api_url,
            current: current.to_owned(),
            exe,
            failures: Mutex::new(HashMap::new()),
            before_swap: None,
        })
    }

    /// 入れ替える直前に、この処理を呼ぶ（新しいバイナリが確かめられたあと、今のバイナリを残す前）。
    pub fn with_before_swap(mut self, f: BeforeSwap) -> Self {
        self.before_swap = Some(f);
        self
    }

    /// 1 回の確認。新しい版に入れ替えたら、その版の文字列を返す。新しい版がなければ None。
    pub async fn check_once(&self) -> Result<Option<String>, String> {
        let body = self
            .client
            .get(&self.api_url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("最新リリースを取得できません: {e}"))?
            .text()
            .await
            .map_err(|e| format!("最新リリースを読めません: {e}"))?;
        let release: Release = serde_json::from_str(&body)
            .map_err(|e| format!("最新リリースの形式が不正です: {e}"))?;
        if release.draft || release.prerelease {
            return Ok(None);
        }
        // 自動更新は、1.2.3 の形の正式版だけ（解釈できない版は入れない）
        let version = release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name)
            .to_owned();
        if offsave_core::version::is_newer(&version, &self.current) != Some(true) {
            return Ok(None);
        }
        if self.given_up(&version) {
            return Ok(None);
        }
        match self.install(&release, &version).await {
            Ok(()) => Ok(Some(version)),
            Err(e) => {
                let n = self.record_failure(&version);
                let msg = if n >= MAX_FAILURES {
                    format!("{e}（{version} は {n} 回失敗したので、再起動まで諦めます）")
                } else {
                    format!("{e}（{version}: {n}/{MAX_FAILURES} 回目の失敗。次の周期で再試行します）")
                };
                Err(msg)
            }
        }
    }

    fn given_up(&self, version: &str) -> bool {
        self.failures
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(version)
            .is_some_and(|n| *n >= MAX_FAILURES)
    }

    fn record_failure(&self, version: &str) -> u32 {
        let mut map = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        let n = map.entry(version.to_owned()).or_insert(0);
        *n += 1;
        *n
    }

    async fn install(&self, release: &Release, version: &str) -> Result<(), String> {
        let (bin_asset, sha_asset) = select_assets(release)
            .ok_or_else(|| format!("{ASSET_NAME} と {ASSET_NAME}.sha256 の資産がありません"))?;

        let sha_text = self.get_bytes(&sha_asset.browser_download_url, 4096).await?;
        let sha_text =
            String::from_utf8(sha_text).map_err(|_| "sha256 が UTF-8 ではありません".to_owned())?;
        let expected = parse_sha256_line(&sha_text).ok_or("sha256 の形式が不正です")?;

        let bytes = self
            .get_bytes(&bin_asset.browser_download_url, MAX_BINARY_BYTES)
            .await?;
        if sha256_hex(&bytes) != expected {
            return Err("sha256 が一致しません".into());
        }

        let tmp = self.tmp_path();
        let result = self.stage_and_swap(&bytes, &tmp, version).await;
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    /// 一時ファイルに書き、想定の版を返すか確かめてから、今のバイナリを `.prev` に残して置き換える。
    async fn stage_and_swap(&self, bytes: &[u8], tmp: &Path, version: &str) -> Result<(), String> {
        write_executable(tmp, bytes).map_err(|e| format!("一時ファイルを書けません: {e}"))?;
        check_version(tmp, version).await?;
        if let Some(before) = &self.before_swap {
            before().await;
        }
        std::fs::copy(&self.exe, self.prev_path())
            .map_err(|e| format!("今のバイナリを残せません: {e}"))?;
        std::fs::rename(tmp, &self.exe).map_err(|e| format!("置き換えられません: {e}"))?;
        Ok(())
    }

    async fn get_bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let res = self
            .client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("ダウンロードできません: {e}"))?;
        if res.content_length().is_some_and(|n| n > limit) {
            return Err("資産が大きすぎます".into());
        }
        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("ダウンロードできません: {e}"))?;
        if bytes.len() as u64 > limit {
            return Err("資産が大きすぎます".into());
        }
        Ok(bytes.to_vec())
    }

    fn tmp_path(&self) -> PathBuf {
        sibling(&self.exe, "update")
    }

    fn prev_path(&self) -> PathBuf {
        sibling(&self.exe, "prev")
    }
}

/// 実行ファイルと同じディレクトリに `<名前>.<suffix>` を作る。
fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "offsave-server".into());
    exe.with_file_name(format!("{name}.{suffix}"))
}

#[cfg(unix)]
fn write_executable(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn write_executable(_path: &Path, _bytes: &[u8]) -> std::io::Result<()> {
    Err(std::io::Error::other("自動更新は Linux だけ対応しています"))
}

/// 新しいバイナリを `--version` で起動して、想定の版の文字列を返すか確かめる。
async fn check_version(path: &Path, version: &str) -> Result<(), String> {
    let mut cmd = tokio::process::Command::new(path);
    cmd.arg("--version").kill_on_drop(true);
    let out = tokio::time::timeout(VERSION_CHECK_TIMEOUT, cmd.output())
        .await
        .map_err(|_| "--version が時間内に返りません".to_owned())?
        .map_err(|e| format!("--version を実行できません: {e}"))?;
    let got = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if !out.status.success() || got != version_line(version) {
        return Err(format!("--version が想定と違います（{got:?}）"));
    }
    Ok(())
}

/// 入れ替えの前の DB のバックアップ（`<data_dir>/backup/offsave-<時刻>.db`）。古いものは [`KEEP_BACKUPS`] 個まで。
async fn backup_db(state: &SharedState) -> Result<PathBuf, String> {
    let dir = state.config.data_dir.join("backup");
    std::fs::create_dir_all(&dir).map_err(|e| format!("バックアップの場所を作れません: {e}"))?;
    let path = dir.join(format!("offsave-{}.db", crate::db::now_ms()));
    // VACUUM INTO は、書き込み中でも一貫した状態のコピーを作る
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_string_lossy().into_owned())
        .execute(&state.pool)
        .await
        .map_err(|e| format!("DB をコピーできません: {e}"))?;
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("offsave-") && n.ends_with(".db"))
        })
        .collect();
    files.sort();
    while files.len() > KEEP_BACKUPS {
        let _ = std::fs::remove_file(files.remove(0));
    }
    Ok(path)
}

/// 定期的に確認する。新しい版に入れ替えたら、再起動を要求する。
/// `api_url` は本番では [`RELEASE_API_URL`]。テストでは偽サーバーを渡す（起動の設定からは渡さない）。
pub fn spawn_update_task(state: SharedState, api_url: String) {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("自動更新を始められません（実行ファイルの場所がわからない）: {e}");
            return;
        }
    };
    let updater = match Updater::new(api_url, env!("CARGO_PKG_VERSION"), exe) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("自動更新を始められません: {e}");
            return;
        }
    };
    let for_backup = state.clone();
    let updater = updater.with_before_swap(Box::new(move || {
        let st = for_backup.clone();
        Box::pin(async move {
            match backup_db(&st).await {
                Ok(p) => tracing::info!("入れ替えの前に DB をバックアップしました（{}）", p.display()),
                Err(e) => tracing::warn!("入れ替えの前のバックアップに失敗しました（入れ替えは続けます）: {e}"),
            }
        })
    }));
    tracing::info!(
        "自動更新を有効にしました（{} 秒後から、{} 分ごとに確認）",
        FIRST_CHECK_DELAY.as_secs(),
        CHECK_INTERVAL.as_secs() / 60
    );
    tokio::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            match updater.check_once().await {
                Ok(Some(version)) => {
                    tracing::info!(
                        "サーバーを {version} に入れ替えました。再起動します（終了コード {RESTART_EXIT_CODE}）"
                    );
                    state.restart.request();
                    return;
                }
                Ok(None) => tracing::debug!("自動更新: 新しい版はありません"),
                Err(e) => tracing::warn!("自動更新に失敗しました: {e}"),
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_is_chosen_by_name() {
        let release: Release = serde_json::from_str(
            r#"{"tag_name":"v1.0.0","assets":[
                {"name":"offsave-server_linux-x64.sha256","browser_download_url":"https://x/sha"},
                {"name":"offsave-server_linux-x64","browser_download_url":"https://x/bin"},
                {"name":"offsave_1.0.0_android-arm64.apk","browser_download_url":"https://x/apk"}
            ]}"#,
        )
        .unwrap();
        let (bin, sha) = select_assets(&release).unwrap();
        assert_eq!(bin.browser_download_url, "https://x/bin");
        assert_eq!(sha.browser_download_url, "https://x/sha");

        let without_sha: Release = serde_json::from_str(
            r#"{"tag_name":"v1.0.0","assets":[{"name":"offsave-server_linux-x64","browser_download_url":"u"}]}"#,
        )
        .unwrap();
        assert!(select_assets(&without_sha).is_none());
    }

    #[test]
    fn sha256_file_and_digest_match() {
        let digest = sha256_hex(b"hello");
        assert_eq!(
            digest,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        let line = format!("{}  offsave-server_linux-x64\n", digest.to_uppercase());
        assert_eq!(parse_sha256_line(&line), Some(digest));
        assert_eq!(parse_sha256_line("abc  x"), None);
        assert_eq!(parse_sha256_line(&"z".repeat(64)), None);
        assert_eq!(parse_sha256_line(""), None);
    }

    #[test]
    fn sibling_paths_sit_next_to_the_binary() {
        let exe = Path::new("/home/u/offsave/bin/offsave-server");
        assert_eq!(sibling(exe, "prev"), Path::new("/home/u/offsave/bin/offsave-server.prev"));
        assert_eq!(sibling(exe, "update"), Path::new("/home/u/offsave/bin/offsave-server.update"));
    }

    #[test]
    fn restart_wakes_waiters_and_is_not_lost() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let r = Restart::default();
            assert!(!r.requested());
            r.request();
            // 要求のあとに待っても、すぐ戻る
            tokio::time::timeout(Duration::from_secs(1), r.wait())
                .await
                .unwrap();
            assert!(r.requested());
        });
    }
}
