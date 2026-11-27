use offsave_core::model::{Limit, Override, Rule, Target};
use sqlx::SqlitePool;

use crate::db::{new_id, now_ms};

#[derive(Debug, Clone)]
pub struct StoredRule {
    pub rule: Rule,
    pub owner_id: String,
    pub author_id: String,
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: String,
    owner_id: String,
    author_id: String,
    name: String,
    targets: String,
    limit_json: String,
    enabled: i64,
}

impl RuleRow {
    fn into_stored(self) -> Option<StoredRule> {
        Some(StoredRule {
            rule: Rule {
                id: self.id,
                name: self.name,
                targets: serde_json::from_str::<Vec<Target>>(&self.targets).ok()?,
                limits: serde_json::from_str::<Vec<Limit>>(&self.limit_json).ok()?,
                enabled: self.enabled != 0,
            },
            owner_id: self.owner_id,
            author_id: self.author_id,
        })
    }
}

pub async fn list(pool: &SqlitePool, owner_id: &str) -> Result<Vec<StoredRule>, sqlx::Error> {
    let rows: Vec<RuleRow> =
        sqlx::query_as("SELECT * FROM rules WHERE owner_id = ? ORDER BY created_at, id")
            .bind(owner_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().filter_map(RuleRow::into_stored).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<StoredRule>, sqlx::Error> {
    let row: Option<RuleRow> = sqlx::query_as("SELECT * FROM rules WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(RuleRow::into_stored))
}

pub async fn insert(
    pool: &SqlitePool,
    owner_id: &str,
    author_id: &str,
    rule: &Rule,
) -> Result<String, sqlx::Error> {
    let id = new_id();
    let now = now_ms();
    sqlx::query(
        "INSERT INTO rules (id, owner_id, author_id, name, targets, limit_json, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(owner_id)
    .bind(author_id)
    .bind(&rule.name)
    .bind(serde_json::to_string(&rule.targets).unwrap_or_default())
    .bind(serde_json::to_string(&rule.limits).unwrap_or_default())
    .bind(rule.enabled as i64)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update(pool: &SqlitePool, id: &str, rule: &Rule) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE rules SET name = ?, targets = ?, limit_json = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&rule.name)
    .bind(serde_json::to_string(&rule.targets).unwrap_or_default())
    .bind(serde_json::to_string(&rule.limits).unwrap_or_default())
    .bind(rule.enabled as i64)
    .bind(now_ms())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM rules WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OverrideRow {
    pub id: String,
    pub owner_id: String,
    pub author_id: String,
    pub date: String,
    pub rule_id: Option<String>,
}

impl OverrideRow {
    pub fn to_override(&self) -> Override {
        Override {
            id: self.id.clone(),
            date: self.date.clone(),
            rule_id: self.rule_id.clone(),
        }
    }
}

/// 古いオーバーライドは返さない（`since_date` 以降だけ）。
pub async fn list_overrides(
    pool: &SqlitePool,
    owner_id: &str,
    since_date: &str,
) -> Result<Vec<OverrideRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM overrides WHERE owner_id = ? AND date >= ? ORDER BY date, id")
        .bind(owner_id)
        .bind(since_date)
        .fetch_all(pool)
        .await
}

pub async fn get_override(pool: &SqlitePool, id: &str) -> Result<Option<OverrideRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM overrides WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn insert_override(
    pool: &SqlitePool,
    owner_id: &str,
    author_id: &str,
    date: &str,
    rule_id: Option<&str>,
) -> Result<String, sqlx::Error> {
    let id = new_id();
    sqlx::query(
        "INSERT INTO overrides (id, owner_id, author_id, date, rule_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(owner_id)
    .bind(author_id)
    .bind(date)
    .bind(rule_id)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn delete_override(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM overrides WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

