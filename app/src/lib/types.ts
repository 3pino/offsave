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

export type Limit =
  | { type: 'window'; days: number; start_min: number; end_min: number }
  | { type: 'daily_total'; seconds: number };

export interface Rule {
  id: string;
  name: string;
  targets: Target[];
  limit: Limit;
  enabled: boolean;
}

export interface RuleInput {
  name: string;
  targets: Target[];
  limit: Limit;
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

export interface UserStatus {
  user: User;
  settings: UserSettings;
  devices: DeviceInfo[];
  rules: RuleStatus[];
  overrides: Override[];
  unlock_until: Ms;
  date: string;
  usage_today: UsageItem[];
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
  | 'rule_changed';

export interface OffsaveEvent {
  id: string;
  user: User;
  device_id: string | null;
  device_name: string | null;
  kind: EventKind;
  message: string;
  at: Ms;
}

/** Android の強制サービスの状態（Kotlin の EnforcerPlugin.status） */
export interface NativeStatus {
  configured: boolean;
  accessibility: boolean;
  deviceAdmin: boolean;
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
