//! 「今日の利用」で初めは隠すアプリ・サイトの一覧。ユーザーごと。

use sqlx::SqlitePool;

use crate::db::now_ms;

pub async fn list(pool: &SqlitePool, owner_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT key FROM hidden_keys WHERE owner_id = ? ORDER BY key")
        .bind(owner_id)
        .fetch_all(pool)
        .await
}

/// `add` を足し（すでにあるものは、そのまま。和集合）、`remove` を外す。
pub async fn change(
    pool: &SqlitePool,
    owner_id: &str,
    author_id: &str,
    add: &[String],
    remove: &[String],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let now = now_ms();
    for key in add {
        sqlx::query("INSERT OR IGNORE INTO hidden_keys (owner_id, key, author_id, created_at) VALUES (?, ?, ?, ?)")
            .bind(owner_id)
            .bind(key)
            .bind(author_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
    }
    for key in remove {
        sqlx::query("DELETE FROM hidden_keys WHERE owner_id = ? AND key = ?")
            .bind(owner_id)
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
}
