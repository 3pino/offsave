//! 環境変数から読み込む設定。

use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// DB を置くディレクトリ（`OFFSAVE_DATA_DIR`）。
    pub data_dir: PathBuf,
    /// 待ち受けるアドレス（`OFFSAVE_BIND`）。本番では Tailscale の IP を指定する。
    pub bind: SocketAddr,
    /// 開発モード（`OFFSAVE_DEV=1`）。whois を使わず、ヘッダーかクエリでユーザーを指定する。
    pub dev: bool,
    /// tailscaled の LocalAPI のソケット（`OFFSAVE_TAILSCALE_SOCKET`）。
    pub tailscale_socket: PathBuf,
    /// 新しいユーザーのタイムゾーンの既定（`OFFSAVE_DEFAULT_TZ`）。
    pub default_timezone: String,
    /// ハートビートがこの時間（ミリ秒）途絶えたら「疑い」にする（`OFFSAVE_SUSPECT_AFTER_SECS`、既定 600 秒）。
    pub suspect_after_ms: i64,
    /// 保護が外れてから友達に知らせるまでの猛予（ミリ秒、`OFFSAVE_PROTECTION_GRACE_SECS`、既定 15 秒）。
    /// アプリの更新や再起動で、一瞬外れてすぐ戻るときに、知らせないため。
    pub protection_grace_ms: u64,
    /// 一時解除の長さ（分）。
    pub unlock_minutes: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());

        let bind = var("OFFSAVE_BIND").unwrap_or_else(|| "127.0.0.1:8090".into());
        let bind = bind
            .parse()
            .map_err(|e| format!("OFFSAVE_BIND が不正です（{bind}）: {e}"))?;
        let suspect_secs = match var("OFFSAVE_SUSPECT_AFTER_SECS") {
            Some(v) => v
                .parse::<i64>()
                .ok()
                .filter(|n| *n >= 1)
                .ok_or_else(|| format!("OFFSAVE_SUSPECT_AFTER_SECS が不正です（{v}）"))?,
            None => 600,
        };

        let grace_secs = match var("OFFSAVE_PROTECTION_GRACE_SECS") {
            Some(v) => v
                .parse::<u64>()
                .map_err(|_| format!("OFFSAVE_PROTECTION_GRACE_SECS が不正です（{v}）"))?,
            None => 15,
        };

        Ok(Self {
            data_dir: var("OFFSAVE_DATA_DIR")
                .unwrap_or_else(|| "./data".into())
                .into(),
            bind,
            dev: var("OFFSAVE_DEV").is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            tailscale_socket: var("OFFSAVE_TAILSCALE_SOCKET")
                .map(PathBuf::from)
                .unwrap_or_else(default_tailscale_socket),
            default_timezone: var("OFFSAVE_DEFAULT_TZ").unwrap_or_else(|| "Asia/Tokyo".into()),
            suspect_after_ms: suspect_secs * 1000,
            protection_grace_ms: grace_secs * 1000,
            unlock_minutes: offsave_core::protocol::UNLOCK_MINUTES,
        })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("offsave.db")
    }
}

fn default_tailscale_socket() -> PathBuf {
    const CANDIDATES: [&str; 2] = [
        "/var/run/tailscale/tailscaled.sock",
        "/var/snap/tailscale/common/socket/tailscaled.sock",
    ];
    CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .unwrap_or_else(|| CANDIDATES[0].into())
}
