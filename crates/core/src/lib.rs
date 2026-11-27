//! offsave の共通部分。サーバー・デスクトップのエージェントが使う。
//!
//! - `model`: ルールなどのデータ型
//! - `time`: 日付の境界・ローカル時刻
//! - `intervals`: 区間の和集合（複数デバイスの同時利用を重複なく数える）
//! - `engine`: 制限の判定（猶予・一時解除を含む）。Android の Kotlin 版と同じ試験データ（tests/vectors）で一致を保つ
//! - `protocol`: デバイスとサーバーの WebSocket のメッセージ
//! - `version`: 版の比較（サーバーの自動更新）

pub mod engine;
pub mod intervals;
pub mod model;
pub mod protocol;
pub mod time;
pub mod version;

/// ミリ秒（Unix 時刻・長さの両方に使う）。
pub type Ms = i64;
