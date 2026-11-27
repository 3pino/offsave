//! アイコン（アプリはデバイスが送ったもの、サイトはサーバーが取ったファビコン）。

use offsave_core::Ms;
use sqlx::SqlitePool;

pub struct IconRow {
    pub mime: String,
    /// None は「取れなかった」の記録
    pub data: Option<Vec<u8>>,
    pub updated_at: Ms,
}

pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<IconRow>, sqlx::Error> {
    let row: Option<(String, Option<Vec<u8>>, i64)> =
        sqlx::query_as("SELECT mime, data, updated_at FROM icons WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(mime, data, updated_at)| IconRow { mime, data, updated_at }))
}

/// 上書きする。`only_if_older_than` を渡すと、それより新しいものがあれば何もしない（アプリのアイコンを、毎回書き直さないため）。
pub async fn put(
    pool: &SqlitePool,
    key: &str,
    mime: &str,
    data: Option<&[u8]>,
    now: Ms,
    only_if_older_than: Option<Ms>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO icons (key, mime, data, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET mime = excluded.mime, data = excluded.data, updated_at = excluded.updated_at
         WHERE icons.updated_at < ?",
    )
    .bind(key)
    .bind(mime)
    .bind(data)
    .bind(now)
    .bind(only_if_older_than.unwrap_or(i64::MAX))
    .execute(pool)
    .await?;
    Ok(())
}
