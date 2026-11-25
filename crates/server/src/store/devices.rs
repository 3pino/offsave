use offsave_core::Ms;
use offsave_core::protocol::{DeviceFlags, DeviceInfo};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::db::{new_id, now_ms};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub platform: String,
    pub token_hash: String,
    pub created_at: i64,
    pub last_seen: i64,
    pub state: String,
    pub flags: String,
    pub foreground: Option<String>,
    pub removed_at: Option<i64>,
}

impl DeviceRow {
    pub fn flags(&self) -> DeviceFlags {
        serde_json::from_str(&self.flags).unwrap_or_default()
    }

    pub fn to_info(&self) -> DeviceInfo {
        DeviceInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            platform: self.platform.clone(),
            state: self.state.clone(),
            last_seen: self.last_seen,
            flags: self.flags(),
            foreground: self.foreground.clone(),
            created_at: self.created_at,
        }
    }
}

fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn new_token() -> String {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).expect("乱数を作れません");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// デバイスを登録する。返すトークンは、この 1 回しか見られない。
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    name: &str,
    platform: &str,
) -> Result<(DeviceRow, String), sqlx::Error> {
    let token = new_token();
    let id = new_id();
    sqlx::query(
        "INSERT INTO devices (id, user_id, name, platform, token_hash, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(name)
    .bind(platform)
    .bind(hash_token(&token))
    .bind(now_ms())
    .execute(pool)
    .await?;
    let row = get(pool, &id).await?.expect("作ったばかり");
    Ok((row, token))
}

pub async fn find_by_token(pool: &SqlitePool, token: &str) -> Result<Option<DeviceRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM devices WHERE token_hash = ? AND removed_at IS NULL")
        .bind(hash_token(token))
        .fetch_optional(pool)
        .await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<DeviceRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM devices WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// 削除していないデバイス。
pub async fn list_for_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<DeviceRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM devices WHERE user_id = ? AND removed_at IS NULL ORDER BY created_at")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

pub async fn heartbeat(
    pool: &SqlitePool,
    id: &str,
    now: Ms,
    flags: &DeviceFlags,
    foreground: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE devices SET last_seen = ?, state = 'online', flags = ?, foreground = ? WHERE id = ?")
        .bind(now)
        .bind(serde_json::to_string(flags).unwrap_or_else(|_| "{}".into()))
        .bind(foreground)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_state(pool: &SqlitePool, id: &str, state: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE devices SET state = ? WHERE id = ?")
        .bind(state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 削除の印を付ける（利用の記録は残す）。
pub async fn remove(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE devices SET removed_at = ?, state = 'offline' WHERE id = ?")
        .bind(now_ms())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// ハートビートが `cutoff` より前で止まっている、接続中のはずのデバイス。
pub async fn stale_online(pool: &SqlitePool, cutoff: Ms) -> Result<Vec<DeviceRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM devices WHERE state = 'online' AND removed_at IS NULL AND last_seen < ?",
    )
    .bind(cutoff)
    .fetch_all(pool)
    .await
}
