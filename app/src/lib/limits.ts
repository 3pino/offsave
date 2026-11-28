import { clock, duration } from './format';
import type { Limit, Rule } from './types';

// ルールの「制限」まわりの、画面に依らない計算（曜日の指定・今日の上限・文章）。

/** 曜日のビット（bit0 = 月 … bit6 = 日） */
export const ALL_DAYS = 0b111_1111;
export const WEEKDAYS = 0b001_1111;
export const WEEKEND = 0b110_0000;

export const WEEK_LABELS = ['月', '火', '水', '木', '金', '土', '日'] as const;

/** `YYYY-MM-DD` の曜日（0 = 月 … 6 = 日） */
export function weekdayOfDate(date: string): number {
  const [y, m, d] = date.split('-').map(Number);
  const sunday0 = new Date(Date.UTC(y, m - 1, d)).getUTCDay();
  return (sunday0 + 6) % 7;
}

/** 曜日の指定の呼び方（毎日・平日・土日・「月水金」） */
export function daysLabel(bits: number): string {
  const b = bits & ALL_DAYS;
  if (b === ALL_DAYS) return '毎日';
  if (b === WEEKDAYS) return '平日';
  if (b === WEEKEND) return '土日';
  return WEEK_LABELS.filter((_, i) => b & (1 << i)).join('');
}

/** 制限の 1 行の説明 */
export function limitText(l: Limit): string {
  if (l.type === 'daily_total') {
    const days = l.days === ALL_DAYS ? '' : `${daysLabel(l.days)}は `;
    return `${days}1日 ${duration(l.seconds)}まで`;
  }
  return `${daysLabel(l.days)} ${clock(l.start_min)}–${clock(l.end_min)} は使えない`;
}

/** 識別子が対象に当たるか。`domain:` はサブドメインも含む（crates/core の `key_matches` と同じ） */
export function keyMatches(target: string, key: string): boolean {
  if (target === key) return true;
  return target.startsWith('domain:') && key.startsWith('domain:') && key.endsWith(`.${target.slice('domain:'.length)}`);
}

/** ルールの対象に、この識別子が当たるか */
export function ruleMatches(rule: Pick<Rule, 'targets'>, key: string): boolean {
  return rule.targets.some((t) => t.keys.some((k) => keyMatches(k, key)));
}

/** 2 つの識別子が、同じものを指しうるか（サブドメインはどちらかがもう一方に含まれれば重なる） */
export const keysOverlap = (a: string, b: string) => keyMatches(a, b) || keyMatches(b, a);

/**
 * 画面に出すルールの名前。名前が空なら、対象から作る（「YouTube」「YouTube ほか3件」）。
 * crates/core の `Rule::display_name` と同じ。
 */
export function ruleDisplayName(rule: Pick<Rule, 'name' | 'targets'>): string {
  const name = rule.name.trim();
  if (name) return name;
  const [first, ...rest] = rule.targets;
  if (!first) return 'ルール';
  return rest.length === 0 ? first.label : `${first.label} ほか${rest.length}件`;
}

/** 論理日（曜日 `weekday`）に効く、1 日の合計の上限（秒）。いくつかあれば一番小さいもの。無ければ null */
export function todayTotal(limits: Limit[], weekday: number): number | null {
  let min: number | null = null;
  for (const l of limits) {
    if (l.type !== 'daily_total' || !(l.days & (1 << weekday))) continue;
    if (min === null || l.seconds < min) min = l.seconds;
  }
  return min;
}

/** 新しい制限の初期値 */
export function newLimit(type: Limit['type']): Limit {
  return type === 'daily_total'
    ? { type: 'daily_total', days: ALL_DAYS, seconds: 3600 }
    : { type: 'window', days: ALL_DAYS, start_min: 22 * 60, end_min: 6 * 60 };
}

/** `HH:MM` → 0 時からの分（不正なら null） */
export function parseTime(s: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(s);
  if (!m) return null;
  const min = Number(m[1]) * 60 + Number(m[2]);
  return Number(m[2]) < 60 && min <= 1440 ? min : null;
}

/** 入力欄の `HH:MM`（1440 は 0:00 に戻す） */
export function timeValue(min: number): string {
  return clock(min);
}

/** 保存してよい制限か。だめなら理由を返す（サーバーも同じ検査をする） */
export function limitError(l: Limit): string | null {
  if ((l.days & ALL_DAYS) === 0) return '曜日を 1 つ以上選んでください';
  if (l.type === 'daily_total') {
    return l.seconds < 0 || l.seconds > 86_400 ? '1 日の上限は 24 時間までです' : null;
  }
  if (l.start_min === l.end_min) return '開始と終了が同じです';
  return null;
}
