use offsave_core::protocol::FriendState;
use sqlx::SqlitePool;

use super::users::UserRow;
use crate::db::now_ms;

fn ordered<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RequestResult {
    Requested,
    /// 相手からの申請があったので、承認になった。
    Accepted,
    AlreadyFriends,
    AlreadyRequested,
}

pub async fn request(
    pool: &SqlitePool,
    me: &str,
    other: &str,
) -> Result<RequestResult, sqlx::Error> {
    let (a, b) = ordered(me, other);
    let existing: Option<(String, i64)> =
        sqlx::query_as("SELECT requested_by, accepted FROM friendships WHERE user_a = ? AND user_b = ?")
            .bind(a)
            .bind(b)
            .fetch_optional(pool)
            .await?;
    match existing {
        Some((_, 1)) => Ok(RequestResult::AlreadyFriends),
        Some((by, _)) if by == me => Ok(RequestResult::AlreadyRequested),
        Some(_) => {
            accept(pool, me, other).await?;
            Ok(RequestResult::Accepted)
        }
        None => {
            sqlx::query(
                "INSERT INTO friendships (user_a, user_b, requested_by, accepted, created_at) VALUES (?, ?, ?, 0, ?)",
            )
            .bind(a)
            .bind(b)
            .bind(me)
            .bind(now_ms())
            .execute(pool)
            .await?;
            Ok(RequestResult::Requested)
        }
    }
}

/// `me` が `other` からの申請を承認する。申請がなければ `false`。
pub async fn accept(pool: &SqlitePool, me: &str, other: &str) -> Result<bool, sqlx::Error> {
    let (a, b) = ordered(me, other);
    let r = sqlx::query(
        "UPDATE friendships SET accepted = 1 WHERE user_a = ? AND user_b = ? AND accepted = 0 AND requested_by <> ?",
    )
    .bind(a)
    .bind(b)
    .bind(me)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// 申請の取り消し・拒否・友達の解消。関係がなければ `None`、あれば解消前に承認済みだったか。
pub async fn remove(
    pool: &SqlitePool,
    me: &str,
    other: &str,
) -> Result<Option<bool>, sqlx::Error> {
    let (a, b) = ordered(me, other);
    let mut tx = pool.begin().await?;
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT accepted FROM friendships WHERE user_a = ? AND user_b = ?")
            .bind(a)
            .bind(b)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((accepted,)) = row else {
        return Ok(None);
    };
    sqlx::query("DELETE FROM friendships WHERE user_a = ? AND user_b = ?")
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;
    // 解消した相手が決めたルールとオーバーライドは消える
    for (owner, author) in [(me, other), (other, me)] {
        sqlx::query("DELETE FROM overrides WHERE owner_id = ? AND author_id = ?")
            .bind(owner)
            .bind(author)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM rules WHERE owner_id = ? AND author_id = ?")
            .bind(owner)
            .bind(author)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Some(accepted == 1))
}

pub async fn is_friend(pool: &SqlitePool, a: &str, b: &str) -> Result<bool, sqlx::Error> {
    let (a, b) = ordered(a, b);
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM friendships WHERE user_a = ? AND user_b = ? AND accepted = 1",
    )
    .bind(a)
    .bind(b)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

/// 承認済みの友達の ID。
pub async fn friend_ids(pool: &SqlitePool, me: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT CASE WHEN user_a = ?1 THEN user_b ELSE user_a END
         FROM friendships WHERE accepted = 1 AND (user_a = ?1 OR user_b = ?1)",
    )
    .bind(me)
    .fetch_all(pool)
    .await
}

pub async fn list(
    pool: &SqlitePool,
    me: &str,
) -> Result<Vec<(UserRow, FriendState)>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        user: UserRow,
        requested_by: String,
        accepted: i64,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT u.*, f.requested_by AS requested_by, f.accepted AS accepted
         FROM friendships f
         JOIN users u ON u.id = CASE WHEN f.user_a = ?1 THEN f.user_b ELSE f.user_a END
         WHERE f.user_a = ?1 OR f.user_b = ?1
         ORDER BY u.display_name",
    )
    .bind(me)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let state = if r.accepted == 1 {
                FriendState::Accepted
            } else if r.requested_by == me {
                FriendState::PendingOut
            } else {
                FriendState::PendingIn
            };
            (r.user, state)
        })
        .collect())
}
