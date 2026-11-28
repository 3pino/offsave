import { addDays } from './format';
import { ruleMatches, weekdayOfDate } from './limits';
import type { Rule, UsageItem } from './types';

// 利用の表示まわりの、画面に依らない計算（週の区切り・合計・ルールごとのまとめ）。

/** これ以下（秒）の利用は、初めは隠す（「続きを表示」で見られる） */
export const SHORT_SECONDS = 60;

export const isSite = (key: string) => key.startsWith('domain:');

/** `date` が属する週の、始まりの日付（`weekStart` は 0 = 月 … 6 = 日） */
export function weekStartOf(date: string, weekStart: number): string {
  const back = (weekdayOfDate(date) - weekStart + 7) % 7;
  return addDays(date, -back);
}

/** 合計（秒）。`pick` が true の識別子だけ数える */
export function sumSeconds(items: UsageItem[], pick: (key: string) => boolean): number {
  let sum = 0;
  for (const u of items) if (pick(u.key)) sum += u.seconds;
  return sum;
}

/**
 * ホームの合計に数える識別子の条件。サイトは、ブラウザのアプリと同時に記録されているので、足さない（二重になる）。
 * `showHidden` が false なら、非表示のアプリも足さない。
 */
export function homePick(isHidden: (key: string) => boolean, showHidden: boolean): (key: string) => boolean {
  return (key) => !isSite(key) && (showHidden || !isHidden(key));
}

/** 利用の詳細を開く対象: アプリ・サイト 1 つ、またはルール 1 つ（カテゴリ全体） */
export type UsageTarget = { kind: 'key'; key: string } | { kind: 'rule'; id: string };

/** 一覧に出す行。アプリ・サイト 1 つか、ルール（まとまり）1 つ */
export type UsageRow =
  | { kind: 'key'; key: string; seconds: number }
  | { kind: 'rule'; rule: Rule; seconds: number; children: UsageItem[] };

const bySeconds = (a: { seconds: number }, b: { seconds: number }) => b.seconds - a.seconds;

/**
 * 一覧の行を作る。`byRule` なら、ルールに入っているものは、ルールの行にまとめる（合計 = 中身の合計）。
 * 1 つのアプリ・サイトは 1 つのルールにしか入らないので、二重には数えない。多い順。
 */
export function buildRows(items: UsageItem[], rules: Rule[], byRule: boolean): UsageRow[] {
  if (!byRule) return items.map((u) => ({ kind: 'key', key: u.key, seconds: u.seconds }) as UsageRow).sort(bySeconds);
  const rows: UsageRow[] = [];
  const grouped = new Map<string, UsageItem[]>();
  for (const u of items) {
    const rule = rules.find((r) => ruleMatches(r, u.key));
    if (!rule) {
      rows.push({ kind: 'key', key: u.key, seconds: u.seconds });
      continue;
    }
    const list = grouped.get(rule.id) ?? [];
    list.push(u);
    grouped.set(rule.id, list);
  }
  for (const rule of rules) {
    const children = (grouped.get(rule.id) ?? []).sort(bySeconds);
    if (children.length > 0) rows.push({ kind: 'rule', rule, seconds: sumSeconds(children, () => true), children });
  }
  return rows.sort(bySeconds);
}

/** 識別子が入っているルール（無ければ null） */
export function ruleOfKey(rules: Rule[], key: string): Rule | null {
  return rules.find((r) => ruleMatches(r, key)) ?? null;
}
