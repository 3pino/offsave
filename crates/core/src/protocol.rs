//! サーバーとクライアントの間の型。REST の応答と、デバイスの WebSocket（`/api/ws/device`）のメッセージ。
//! JSON は snake_case。

use serde::{Deserialize, Serialize};

use crate::Ms;
use crate::intervals::Segment;
use crate::model::{Override, Rule};
use crate::time::Clock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub login_name: String,
    pub display_name: String,
}

/// デバイスが送る保護の状態。どれかが外れたら友達に通知する。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceFlags {
    pub accessibility: bool,
    pub device_admin: bool,
    pub battery_unrestricted: bool,
}

impl DeviceFlags {
    /// 制限を強制できる状態か。
    pub fn protected(&self) -> bool {
        self.accessibility && self.device_admin
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    /// `online`（接続中）/ `suspect`（ハートビートが途絶えた）/ `offline`（正常に終了した）/ `new`（まだ接続していない）
    pub state: String,
    pub last_seen: Ms,
    pub flags: DeviceFlags,
    pub foreground: Option<String>,
    pub created_at: Ms,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSettings {
    /// IANA のタイムゾーン（例: `Asia/Tokyo`）。
    pub timezone: String,
    pub day_boundary_min: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleStatus {
    pub rule: Rule,
    pub author: User,
    pub used_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageItem {
    pub key: String,
    pub seconds: u32,
}

/// 識別子の表示名（例: `android:com.example.game` → 「ゲーム」）。デバイスが送る。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyLabel {
    pub key: String,
    pub label: String,
}

/// アプリのアイコン（PNG を base64 にしたもの）。デバイスが送る。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyIcon {
    pub key: String,
    pub png: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserStatus {
    pub user: User,
    pub settings: UserSettings,
    pub devices: Vec<DeviceInfo>,
    pub rules: Vec<RuleStatus>,
    pub overrides: Vec<Override>,
    pub unlock_until: Ms,
    /// 今日（論理日）の日付。
    pub date: String,
    pub usage_today: Vec<UsageItem>,
    /// 利用したアプリの表示名。
    #[serde(default)]
    pub labels: Vec<KeyLabel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendState {
    /// 相手が申請してきた（承認待ち）。
    PendingIn,
    /// 自分が申請した。
    PendingOut,
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Friend {
    pub user: User,
    pub state: FriendState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// 本人が制限を一時解除した（本文は本人が書いた理由）。
    TempUnlock,
    /// ハートビートが途絶えた。
    DeviceOffline,
    DeviceOnline,
    DeviceAdded,
    DeviceRemoved,
    /// Accessibility や Device Admin が外れた。
    ProtectionDisabled,
    ProtectionRestored,
    /// 設定画面で offsave の無効化・アンインストールを試みた。
    TamperAttempt,
    /// 制限に達した（猶予に入った）。
    LimitReached,
    FriendAdded,
    FriendRemoved,
    SettingsChanged,
    /// 友達がルールやオーバーライドを変えた。
    RuleChanged,
    /// 対応していないブラウザを使った（URL を見られないので、サイトの制限が効かない）。
    UnsupportedBrowser,
    /// デバイスの時計がサーバーと大きくずれている。
    ClockSkew,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    /// 誰についてのイベントか。
    pub user: User,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub kind: EventKind,
    pub message: String,
    pub at: Ms,
}

/// サーバーがデバイスに渡す方針。ハートビートの返事として毎回送る。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub server_time: Ms,
    pub clock: Clock,
    pub rules: Vec<Rule>,
    pub overrides: Vec<Override>,
    /// 一時解除の終わり（全デバイス共通）。無ければ 0。
    pub unlock_until: Ms,
    pub last_unlock_end: Ms,
    /// 一時解除の長さ（分）。
    pub unlock_minutes: u32,
    /// ほかのデバイスの、今日の利用区間（識別子ごとにまとめたもの）。
    pub others: Vec<Segment>,
}

/// デバイス → サーバー。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceMsg {
    Heartbeat {
        flags: DeviceFlags,
        foreground: Option<String>,
        app_version: String,
        /// 最後に受け取ったイベントの ID（これより新しいものを送り直す）。
        last_event_id: Option<String>,
    },
    /// 終わった区間（まだ続いている区間は、送った時点で区切る）。`labels` は、初めて送るアプリの表示名。
    Usage {
        segments: Vec<Segment>,
        #[serde(default)]
        labels: Vec<KeyLabel>,
    },
    /// アプリのアイコン（友達の画面で、アプリ名の横に出す）。新しいアプリを使ったときだけ送る。
    Icons { icons: Vec<KeyIcon> },
    /// 本人の一時解除。`reason` が友達への通知の本文になる。
    Unlock {
        reason: String,
        started_at: Ms,
        until: Ms,
    },
    Report { kind: EventKind, message: String },
    /// 正常な終了。
    Bye,
}

/// サーバー → デバイス。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Policy(Policy),
    /// 友達（と自分）についてのイベント。
    Event { event: Event },
    Error { message: String },
}

/// 1 回の一時解除の長さ（分）の既定。
pub const UNLOCK_MINUTES: u32 = 15;
