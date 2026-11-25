use offsave_core::Ms;
use offsave_core::intervals::Segment;
use sqlx::SqlitePool;

pub async fn insert_segments(
    pool: &SqlitePool,
    device_id: &str,
    user_id: &str,
    segments: &[Segment],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for s in segments {
        // 同じ開始の区間が再送されたら、終わりを延ばす
        sqlx::query(
            "INSERT INTO usage_segments (device_id, user_id, key, start, \"end\") VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(device_id, key, start) DO UPDATE SET \"end\" = MAX(\"end\", excluded.\"end\")",
        )
        .bind(device_id)
        .bind(user_id)
        .bind(&s.key)
        .bind(s.start)
        .bind(s.end)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// `[lo, hi)` に重なるユーザーの区間。`exclude_device` のものは除く。
pub async fn segments_of_user(
    pool: &SqlitePool,
    user_id: &str,
    exclude_device: Option<&str>,
    lo: Ms,
    hi: Ms,
) -> Result<Vec<Segment>, sqlx::Error> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT key, start, \"end\" FROM usage_segments
         WHERE user_id = ? AND \"end\" > ? AND start < ? AND (? IS NULL OR device_id <> ?)",
    )
    .bind(user_id)
    .bind(lo)
    .bind(hi)
    .bind(exclude_device)
    .bind(exclude_device)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(key, start, end)| Segment { key, start, end })
        .collect())
}
