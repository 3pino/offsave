use offsave_core::protocol::{User, UserSettings};
use sqlx::SqlitePool;

use crate::db::{new_id, now_ms};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub login_name: String,
    pub display_name: String,
    pub timezone: String,
    pub day_boundary_min: i64,
    pub week_start: i64,
    pub created_at: i64,
}

impl UserRow {
    pub fn to_user(&self) -> User {
        User {
            id: self.id.clone(),
            login_name: self.login_name.clone(),
            display_name: self.display_name.clone(),
        }
    }

    pub fn settings(&self) -> UserSettings {
        UserSettings {
            timezone: self.timezone.clone(),
            day_boundary_min: self.day_boundary_min as u32,
            week_start: self.week_start.clamp(0, 6) as u8,
        }
    }
}

/// なければ作る。表示名が変わっていれば直す。
pub async fn get_or_create(
    pool: &SqlitePool,
    login_name: &str,
    display_name: &str,
    default_timezone: &str,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, login_name, display_name, timezone, created_at)
         VALUES (?, ?, ?, ?, ?) ON CONFLICT(login_name) DO NOTHING",
    )
    .bind(new_id())
    .bind(login_name)
    .bind(display_name)
    .bind(default_timezone)
    .bind(now_ms())
    .execute(pool)
    .await?;
    sqlx::query("UPDATE users SET display_name = ? WHERE login_name = ? AND display_name <> ?")
        .bind(display_name)
        .bind(login_name)
        .bind(display_name)
        .execute(pool)
        .await?;
    sqlx::query_as("SELECT * FROM users WHERE login_name = ?")
        .bind(login_name)
        .fetch_one(pool)
        .await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn by_login(pool: &SqlitePool, login_name: &str) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE login_name = ?")
        .bind(login_name)
        .fetch_optional(pool)
        .await
}

pub async fn update_settings(
    pool: &SqlitePool,
    id: &str,
    timezone: &str,
    day_boundary_min: u32,
    week_start: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET timezone = ?, day_boundary_min = ?, week_start = ? WHERE id = ?")
        .bind(timezone)
        .bind(day_boundary_min as i64)
        .bind(week_start as i64)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 自分以外の、offsave を使っている人（友達の候補を探すのに使う）。
pub async fn others(pool: &SqlitePool, me: &str) -> Result<Vec<UserRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM users WHERE id != ? ORDER BY display_name, login_name")
        .bind(me)
        .fetch_all(pool)
        .await
}
