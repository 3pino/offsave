use offsave_core::Ms;
use offsave_core::protocol::{Event, EventKind, User};
use sqlx::SqlitePool;

use crate::db::{new_id, now_ms};

pub fn kind_to_str(kind: EventKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn kind_from_str(s: &str) -> Option<EventKind> {
    serde_json::from_value(serde_json::Value::String(s.to_owned())).ok()
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: String,
    kind: String,
    message: String,
    at: i64,
    device_id: Option<String>,
    device_name: Option<String>,
    user_id: String,
    login_name: String,
    display_name: String,
}

impl EventRow {
    fn into_event(self) -> Option<Event> {
        Some(Event {
            id: self.id,
            user: User {
                id: self.user_id,
                login_name: self.login_name,
                display_name: self.display_name,
            },
            device_id: self.device_id,
            device_name: self.device_name,
            kind: kind_from_str(&self.kind)?,
            message: self.message,
            at: self.at,
        })
    }
}

const SELECT: &str = "SELECT e.id, e.kind, e.message, e.at, e.device_id, d.name AS device_name,
        u.id AS user_id, u.login_name, u.display_name
     FROM events e JOIN users u ON u.id = e.user_id LEFT JOIN devices d ON d.id = e.device_id";

pub async fn insert(
    pool: &SqlitePool,
    user_id: &str,
    device_id: Option<&str>,
    kind: EventKind,
    message: &str,
) -> Result<Event, sqlx::Error> {
    let id = new_id();
    sqlx::query("INSERT INTO events (id, user_id, device_id, kind, message, at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(user_id)
        .bind(device_id)
        .bind(kind_to_str(kind))
        .bind(message)
        .bind(now_ms())
        .execute(pool)
        .await?;
    let row: EventRow = sqlx::query_as(&format!("{SELECT} WHERE e.id = ?"))
        .bind(&id)
        .fetch_one(pool)
        .await?;
    Ok(row.into_event().expect("今書いた種類は読める"))
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

/// `user_ids` についてのイベント。`after` があればそれより新しいものを古い順に、なければ最新から新しい順に。
pub async fn list(
    pool: &SqlitePool,
    user_ids: &[String],
    after: Option<&str>,
    limit: i64,
) -> Result<Vec<Event>, sqlx::Error> {
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    let ph = placeholders(user_ids.len());
    let sql = match after {
        Some(_) => format!("{SELECT} WHERE e.user_id IN ({ph}) AND e.id > ? ORDER BY e.id ASC LIMIT ?"),
        None => format!("{SELECT} WHERE e.user_id IN ({ph}) ORDER BY e.id DESC LIMIT ?"),
    };
    let mut q = sqlx::query_as::<_, EventRow>(&sql);
    for id in user_ids {
        q = q.bind(id);
    }
    if let Some(a) = after {
        q = q.bind(a);
    }
    let rows = q.bind(limit).fetch_all(pool).await?;
    Ok(rows.into_iter().filter_map(EventRow::into_event).collect())
}

/// 一時解除を記録する。同じ解除（デバイス + 開始時刻）の再送なら何もせず `false`。
pub async fn insert_unlock(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    reason: &str,
    started_at: Ms,
    until: Ms,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "INSERT INTO unlocks (id, user_id, device_id, reason, started_at, until) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(device_id, started_at) DO NOTHING",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(device_id)
    .bind(reason)
    .bind(started_at)
    .bind(until)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// （今の一時解除の終わり or 0、直近に終わった一時解除の終わり or 0）。
pub async fn unlock_state(
    pool: &SqlitePool,
    user_id: &str,
    now: Ms,
) -> Result<(Ms, Ms), sqlx::Error> {
    let active: Option<i64> =
        sqlx::query_scalar("SELECT MAX(until) FROM unlocks WHERE user_id = ? AND until > ?")
            .bind(user_id)
            .bind(now)
            .fetch_one(pool)
            .await?;
    let last_end: Option<i64> =
        sqlx::query_scalar("SELECT MAX(until) FROM unlocks WHERE user_id = ? AND until <= ?")
            .bind(user_id)
            .bind(now)
            .fetch_one(pool)
            .await?;
    Ok((active.unwrap_or(0), last_end.unwrap_or(0)))
}
