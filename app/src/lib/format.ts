import type { EventKind, Ms } from './types';

/** 秒 → 「1時間 5分」「30秒」 */
export function duration(seconds: number): string {
  if (seconds < 60) return `${seconds}秒`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return h > 0 ? `${h}時間${m > 0 ? ` ${m}分` : ''}` : `${m}分`;
}

/** 0 時からの分 → `HH:MM`（1440 は 00:00） */
export function clock(min: number): string {
  const m = min === 1440 ? 0 : min;
  return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
}

/** `android:com.example.app` → `com.example.app` */
export function keyName(key: string): string {
  return key.replace(/^(android|win|linux|domain):/, '');
}

export function dateTime(ms: Ms): string {
  const d = new Date(ms);
  const today = new Date();
  const hm = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  return d.toDateString() === today.toDateString() ? hm : `${d.getMonth() + 1}/${d.getDate()} ${hm}`;
}

/** 「3分前」など */
export function ago(ms: Ms, now = Date.now()): string {
  const s = Math.max(0, Math.floor((now - ms) / 1000));
  if (s < 60) return `${s}秒前`;
  if (s < 3600) return `${Math.floor(s / 60)}分前`;
  if (s < 86400) return `${Math.floor(s / 3600)}時間前`;
  return `${Math.floor(s / 86400)}日前`;
}

export const EVENT_LABEL: Record<EventKind, string> = {
  temp_unlock: '制限を一時解除しました',
  device_offline: '連絡が途絶えました',
  device_online: '再び接続しました',
  device_added: 'デバイスが追加されました',
  device_removed: 'デバイスが削除されました',
  protection_disabled: '保護が無効になりました',
  protection_restored: '保護が有効になりました',
  tamper_attempt: '無効化・アンインストールを試みました',
  limit_reached: '制限に達しました',
  friend_added: '友達になりました',
  friend_removed: '友達を解消しました',
  settings_changed: '設定を変更しました',
  rule_changed: 'ルールが変わりました',
  unsupported_browser: '対応していないブラウザを使いました',
  clock_skew: '時計がずれています',
  protection_paused: '保護を一時停止・再開しました',
};

/** 警告として強調するイベント */
export const ALERT_KINDS: EventKind[] = [
  'temp_unlock',
  'device_offline',
  'protection_disabled',
  'tamper_attempt',
  'device_removed',
  'unsupported_browser',
  'clock_skew',
];

/** 論理日の日付に日数を足す（`YYYY-MM-DD`） */
export function addDays(date: string, n: number): string {
  const [y, m, d] = date.split('-').map(Number);
  const t = new Date(Date.UTC(y, m - 1, d + n));
  return `${t.getUTCFullYear()}-${String(t.getUTCMonth() + 1).padStart(2, '0')}-${String(t.getUTCDate()).padStart(2, '0')}`;
}
