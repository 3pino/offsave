//! ハートビートの見張り。途絶えたデバイスを「疑い」にして、友達に知らせる。

use std::sync::Arc;
use std::time::Duration;

use offsave_core::Ms;
use offsave_core::protocol::EventKind;

use crate::db::now_ms;
use crate::service::emit;
use crate::state::AppState;
use crate::store::devices;

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            if let Err(e) = tick(&state, now_ms()).await {
                tracing::warn!("見張りに失敗しました: {}", e.message);
            }
        }
    });
}

/// 1 回分の見張り。`now` を渡せるようにしてある（試験用）。
pub async fn tick(state: &AppState, now: Ms) -> Result<(), crate::error::AppError> {
    let stale = devices::stale_online(&state.pool, now - state.config.suspect_after_ms).await?;
    for d in stale {
        devices::set_state(&state.pool, &d.id, "suspect").await?;
        let minutes = ((now - d.last_seen) / 60_000).max(1);
        emit(
            state,
            &d.user_id,
            Some(&d.id),
            EventKind::DeviceOffline,
            &format!("{} からの連絡が {minutes} 分以上途絶えています", d.name),
        )
        .await?;
    }
    Ok(())
}
