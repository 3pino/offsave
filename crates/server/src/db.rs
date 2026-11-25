//! SQLite の接続と、ID・時刻の生成。

use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use offsave_core::Ms;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use ulid::{Generator, Ulid};

pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str("sqlite://")?
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(10));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// 同じミリ秒の中でも順番が保たれる ULID（イベントの並べ替えに使う）。
pub fn new_id() -> String {
    static GENERATOR: OnceLock<Mutex<Generator>> = OnceLock::new();
    let mut g = GENERATOR
        .get_or_init(|| Mutex::new(Generator::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    g.generate().unwrap_or_else(|_| Ulid::new()).to_string()
}

pub fn now_ms() -> Ms {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as Ms)
        .unwrap_or_default()
}
