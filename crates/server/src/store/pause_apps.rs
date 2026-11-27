//! Accessibility を拒むアプリ（決済アプリなど）の一覧。ユーザーごと。

use sqlx::SqlitePool;

use crate::db::now_ms;

pub async fn list(pool: &SqlitePool, owner_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT key FROM pause_apps WHERE owner_id = ? ORDER BY key")
        .bind(owner_id)
        .fetch_all(pool)
        .await
}

/// 一覧を、渡したものに置き換える。
pub async fn replace(pool: &SqlitePool, owner_id: &str, author_id: &str, keys: &[String]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM pause_apps WHERE owner_id = ?")
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;
    let now = now_ms();
    for key in keys {
        sqlx::query("INSERT INTO pause_apps (owner_id, key, author_id, created_at) VALUES (?, ?, ?, ?)")
            .bind(owner_id)
            .bind(key)
            .bind(author_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
}
