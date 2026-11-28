import { describe, expect, it } from 'vitest';
import { keyMatches, keysOverlap, ruleDisplayName } from './limits';
import { buildRows, homePick, isLimited, ruleOfKey, sumSeconds, weekStartOf } from './usage';
import type { Rule } from './types';

const rule = (id: string, name: string, targets: [string, string][]): Rule => ({
  id,
  name,
  targets: targets.map(([label, key]) => ({ label, keys: [key] })),
  limits: [{ type: 'daily_total', days: 127, seconds: 3600 }],
  enabled: true,
});

describe('ルールの名前', () => {
  it('名前が空なら、対象から付ける', () => {
    expect(ruleDisplayName(rule('1', '', [['YouTube', 'android:com.youtube']]))).toBe('YouTube');
    expect(
      ruleDisplayName(rule('1', ' ', [['YouTube', 'android:a'], ['youtube.com', 'domain:youtube.com'], ['Twitch', 'android:b'], ['Netflix', 'android:c']])),
    ).toBe('YouTube ほか3件');
    expect(ruleDisplayName(rule('1', '  動画 ', [['YouTube', 'android:a']]))).toBe('動画');
  });
});

describe('識別子の重なり', () => {
  it('サイトはサブドメインも含む', () => {
    expect(keyMatches('domain:youtube.com', 'domain:m.youtube.com')).toBe(true);
    expect(keyMatches('domain:m.youtube.com', 'domain:youtube.com')).toBe(false);
    expect(keysOverlap('domain:m.youtube.com', 'domain:youtube.com')).toBe(true);
    expect(keysOverlap('domain:youtube.com', 'domain:notyoutube.com')).toBe(false);
    expect(keysOverlap('android:a', 'domain:a')).toBe(false);
  });
});

describe('週', () => {
  it('週の始まりの日付', () => {
    // 2026-11-25 は水曜
    expect(weekStartOf('2026-11-25', 0)).toBe('2026-11-23'); // 月始まり
    expect(weekStartOf('2026-11-25', 6)).toBe('2026-11-22'); // 日始まり
    expect(weekStartOf('2026-11-23', 0)).toBe('2026-11-23'); // 始まりの日そのもの
    expect(weekStartOf('2026-11-22', 0)).toBe('2026-11-16'); // 日曜は、月始まりでは前の週の最後
    expect(weekStartOf('2026-12-01', 2)).toBe('2026-11-25'); // 月をまたぐ
  });
});

describe('合計と一覧', () => {
  const items = [
    { key: 'android:game', seconds: 600 },
    { key: 'android:secret', seconds: 300 },
    { key: 'domain:example.com', seconds: 120 },
    { key: 'android:youtube', seconds: 900 },
    { key: 'domain:m.youtube.com', seconds: 200 },
  ];
  const hidden = (k: string) => k === 'android:secret';

  it('合計はアプリだけ（サイトは二重になる）。非表示は、見せる設定のときだけ足す', () => {
    expect(sumSeconds(items, homePick(hidden, false))).toBe(600 + 900);
    expect(sumSeconds(items, homePick(hidden, true))).toBe(600 + 300 + 900);
  });

  it('ルールごとに、中身をまとめる', () => {
    const rules = [rule('r1', '', [['YouTube', 'android:youtube'], ['youtube.com', 'domain:youtube.com']])];
    const rows = buildRows(items, rules, true);
    expect(rows.map((r) => (r.kind === 'rule' ? `rule:${r.seconds}` : r.key))).toEqual([
      'rule:1100',
      'android:game',
      'android:secret',
      'domain:example.com',
    ]);
    const group = rows[0];
    expect(group.kind === 'rule' && group.children.map((c) => c.key)).toEqual(['android:youtube', 'domain:m.youtube.com']);
    // まとめないときは、1 つずつ多い順
    expect(buildRows(items, rules, false).map((r) => (r as { key: string }).key)).toEqual([
      'android:youtube',
      'android:game',
      'android:secret',
      'domain:m.youtube.com',
      'domain:example.com',
    ]);
  });

  it('対象が 1 つだけのルールは、まとめない。制限の対象は limited', () => {
    const rules = [rule('r1', '', [['YouTube', 'android:youtube']]), { ...rule('r2', '', [['ゲーム', 'android:game']]), enabled: false }];
    const rows = buildRows(items, rules, true);
    expect(rows.every((r) => r.kind === 'key')).toBe(true);
    const limited = Object.fromEntries(rows.map((r) => [(r as { key: string }).key, r.limited]));
    expect(limited['android:youtube']).toBe(true);
    expect(limited['android:game']).toBe(false); // 無効のルール
    expect(limited['android:secret']).toBe(false);
    expect(isLimited(rules, 'domain:m.youtube.com')).toBe(false);
  });

  it('入っているルールを探す', () => {
    const rules = [rule('r1', '', [['YouTube', 'domain:youtube.com']])];
    expect(ruleOfKey(rules, 'domain:m.youtube.com')?.id).toBe('r1');
    expect(ruleOfKey(rules, 'android:game')).toBeNull();
  });
});
