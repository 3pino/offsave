import { describe, expect, it } from 'vitest';
import { ALL_DAYS, WEEKDAYS, WEEKEND, daysLabel, limitError, limitText, newLimit, parseTime, todayTotal, weekdayOfDate } from './limits';
import type { Limit } from './types';

describe('曜日', () => {
  it('日付の曜日は月曜が 0', () => {
    expect(weekdayOfDate('2026-11-25')).toBe(2); // 水
    expect(weekdayOfDate('2026-11-28')).toBe(5); // 土
    expect(weekdayOfDate('2026-11-29')).toBe(6); // 日
    expect(weekdayOfDate('2026-11-30')).toBe(0); // 月
  });

  it('指定の呼び方', () => {
    expect(daysLabel(ALL_DAYS)).toBe('毎日');
    expect(daysLabel(WEEKDAYS)).toBe('平日');
    expect(daysLabel(WEEKEND)).toBe('土日');
    expect(daysLabel(0b000_0101)).toBe('月水');
  });
});

describe('制限の文章', () => {
  it('合計と時間帯', () => {
    expect(limitText({ type: 'daily_total', days: ALL_DAYS, seconds: 3600 })).toBe('1日 1時間まで');
    expect(limitText({ type: 'daily_total', days: WEEKEND, seconds: 7200 })).toBe('土日は 1日 2時間まで');
    expect(limitText({ type: 'window', days: ALL_DAYS, start_min: 360, end_min: 420 })).toBe('毎日 06:00–07:00 は使えない');
    expect(limitText({ type: 'window', days: WEEKDAYS, start_min: 900, end_min: 1380 })).toBe('平日 15:00–23:00 は使えない');
    // 24:00 までの時間帯は、終わりを 00:00 と書く
    expect(limitText({ type: 'window', days: ALL_DAYS, start_min: 1320, end_min: 1440 })).toBe('毎日 22:00–00:00 は使えない');
  });
});

describe('今日の上限', () => {
  const limits: Limit[] = [
    { type: 'daily_total', days: WEEKDAYS, seconds: 7200 },
    { type: 'daily_total', days: WEEKEND, seconds: 600 },
    { type: 'window', days: ALL_DAYS, start_min: 0, end_min: 60 },
  ];
  it('その曜日に効く合計だけを見る', () => {
    expect(todayTotal(limits, 2)).toBe(7200);
    expect(todayTotal(limits, 5)).toBe(600);
  });
  it('複数効くときは小さいほう', () => {
    expect(todayTotal([...limits, { type: 'daily_total', days: ALL_DAYS, seconds: 300 }], 2)).toBe(300);
  });
  it('合計が無ければ null', () => {
    expect(todayTotal([limits[2]], 2)).toBeNull();
  });
});

describe('入力', () => {
  it('時刻の解釈', () => {
    expect(parseTime('06:30')).toBe(390);
    expect(parseTime('6:05')).toBe(365);
    expect(parseTime('24:00')).toBe(1440);
    expect(parseTime('24:01')).toBeNull();
    expect(parseTime('12:60')).toBeNull();
    expect(parseTime('')).toBeNull();
  });

  it('制限の検査（サーバーと同じ規則）', () => {
    expect(limitError(newLimit('daily_total'))).toBeNull();
    expect(limitError(newLimit('window'))).toBeNull();
    expect(limitError({ type: 'window', days: 0, start_min: 0, end_min: 60 })).toMatch(/曜日/);
    expect(limitError({ type: 'window', days: ALL_DAYS, start_min: 60, end_min: 60 })).toMatch(/同じ/);
    expect(limitError({ type: 'daily_total', days: ALL_DAYS, seconds: 86_401 })).toMatch(/24 時間/);
  });
});
