// サーバーの型（crates/core の protocol・model と同じ JSON）

export type Ms = number;

export interface User {
  id: string;
  login_name: string;
  display_name: string;
}

export interface Target {
  label: string;
  keys: string[];
}

/**
 * 制限 1 つ。ルールは複数持ち、どれか 1 つでも当たっていれば使えない。
 * `days` は曜日のビット（bit0 = 月 … bit6 = 日）。window は「始まる日」、daily_total は「論理日の曜日」。
 */
export type Limit =
  | { type: 'window'; days: number; start_min: number; end_min: number }
  | { type: 'daily_total'; days: number; seconds: number };

export interface Rule {
  id: string;
  name: string;
  targets: Target[];
  limits: Limit[];
  enabled: boolean;
}

export interface RuleInput {
  name: string;
  targets: Target[];
  limits: Limit[];
  enabled: boolean;
}

export interface Override {
  id: string;
  date: string;
  rule_id: string | null;
}

export interface DeviceFlags {
  accessibility: boolean;
  device_admin: boolean;
  battery_unrestricted: boolean;
  /** 友達が決めたアプリの間だけ、Accessibility を自分で外している */
  paused?: boolean;
}

export interface DeviceInfo {
  id: string;
  name: string;
  platform: string;
  state: 'online' | 'suspect' | 'offline' | 'new';
  last_seen: Ms;
  flags: DeviceFlags;
  foreground: string | null;
  created_at: Ms;
}

export interface UserSettings {
  timezone: string;
  day_boundary_min: number;
}

export interface RuleStatus {
  rule: Rule;
  author: User;
  used_seconds: number;
}

export interface UsageItem {
  key: string;
  seconds: number;
}

export interface KeyLabel {
  key: string;
  label: string;
}

export interface UserStatus {
  user: User;
  settings: UserSettings;
  devices: DeviceInfo[];
  rules: RuleStatus[];
  overrides: Override[];
  unlock_until: Ms;
  date: string;
  usage_today: UsageItem[];
  labels: KeyLabel[];
  /** Accessibility を拒むアプリ（決済アプリなど）。前面にある間だけ、デバイスは Accessibility を外す。友達だけが決められる */
  pause_apps: string[];
}

export type FriendState = 'pending_in' | 'pending_out' | 'accepted';

export interface Friend {
  user: User;
  state: FriendState;
}

export type EventKind =
  | 'temp_unlock'
  | 'device_offline'
  | 'device_online'
  | 'device_added'
  | 'device_removed'
  | 'protection_disabled'
  | 'protection_restored'
  | 'tamper_attempt'
  | 'limit_reached'
  | 'friend_added'
  | 'friend_removed'
  | 'settings_changed'
  | 'rule_changed'
  | 'unsupported_browser'
  | 'clock_skew'
  | 'protection_paused';

export interface OffsaveEvent {
  id: string;
  user: User;
  device_id: string | null;
  device_name: string | null;
  kind: EventKind;
  message: string;
  at: Ms;
}

/** 「Android の設定」から直接開ける、このアプリの設定の項目（Kotlin の AndroidSettings.intents と対応） */
export type AndroidSettingKind = 'notifications' | 'permissions' | 'data' | 'battery' | 'open_by_default';

/** Android の強制サービスの状態（Kotlin の EnforcerPlugin.status） */
export interface NativeStatus {
  configured: boolean;
  accessibility: boolean;
  deviceAdmin: boolean;
  /** 友達が決めたアプリのために、Accessibility を自分で外している間 */
  paused: boolean;
  /** 設定の書き込みの権限（adb で 1 回付ける） */
  canWriteSecure: boolean;
  /** Shizuku が動いている（これで権限を付けられる） */
  shizuku: boolean;
  /** 使用状況へのアクセス */
  usageAccess: boolean;
  /** 友達が決めた、Accessibility を拒むアプリの数 */
  pauseApps: number;
  packageName: string;
  batteryUnrestricted: boolean;
  notifications: boolean;
  serviceRunning: boolean;
  connected: boolean;
  lastError: string | null;
  lastPolicyAt: Ms;
  unlockUntil: Ms;
  foreground: string | null;
  ruleCount: number;
}

export interface InstalledApp {
  package: string;
  label: string;
}
