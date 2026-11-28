<script lang="ts" module>
  import type { UsageItem } from '../lib/types';

  /** 選んでいる日（または時間）の利用。`children` で、一覧を出すのに使う */
  export interface Selection {
    /** 選んだ日・時間の、全部の利用（`pick` で絞る前） */
    items: UsageItem[];
    /** 一覧の見出し（「今日の利用（2026-11-28）」など） */
    title: string;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import SegmentedButton from '@3pino/ui-kit-web/SegmentedButton.svelte';
  import { addDays, duration } from '../lib/format';
  import { WEEK_LABELS } from '../lib/limits';
  import { session } from '../lib/session.svelte';
  import type { UsageHistory, UserStatus } from '../lib/types';
  import { sumSeconds, weekStartOf } from '../lib/usage';
  import { usagePrefs, type ChartMode } from '../lib/usagePrefs.svelte';
  import UsageChart, { type Bar } from './UsageChart.svelte';

  // 利用時間のまとめ: 1 日ごと（週）・1 時間ごと（1 日）の棒グラフと、選んだ日・時間の合計。
  // 棒を押すと、その日・時間に絞る（初めは今日）。`children` に、絞った利用を渡す（一覧用）。
  // `pick` が true の識別子だけを数える（ホームは「アプリだけ・非表示を除く」、詳細は「その項目だけ」）
  let {
    userId,
    status,
    pick,
    children,
  }: {
    userId: string;
    status: UserStatus;
    pick: (key: string) => boolean;
    children?: Snippet<[Selection]>;
  } = $props();

  const api = $derived(session.api!);

  const today = $derived(status.date);
  const weekFrom = $derived(weekStartOf(today, status.settings.week_start));
  const timezone = $derived(status.settings.timezone);

  let week = $state.raw<UsageHistory | null>(null);
  let hours = $state.raw<UsageHistory | null>(null);
  /** 見ている日（1 日ごとの棒を押すと変わる。1 時間ごとのグラフは、この日のもの） */
  let pickedDate = $state<string | null>(null);
  /** 1 時間ごとのグラフで、押した棒 */
  let pickedHour = $state<number | null>(null);

  const date = $derived(pickedDate ?? today);
  const mode = $derived(usagePrefs.chart);

  // 履歴を取る（開いている間、15 秒ごとに取り直す）
  $effect(() => {
    const uid = userId;
    const from = weekFrom;
    const d = date;
    const hourly = mode === 'hour';
    let alive = true;
    const load = async () => {
      try {
        const w = await api.usage(uid, from, 7);
        if (alive) week = w;
        if (hourly) {
          const h = await api.usage(uid, d, 1, true);
          if (alive) hours = h;
        }
      } catch {
        /* 取れなければ、前の表示のまま（通知は、状態の取得が出す） */
      }
    };
    void load();
    const t = setInterval(load, 15_000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  });

  // 今日は、状態（3 秒ごと）の値を使う。履歴は 15 秒ごとなので、今日の棒と合計がずれないように
  function dayItems(b: { date: string; items: UsageItem[] }): UsageItem[] {
    return b.date === today ? status.usage_today : b.items;
  }
  const daySeconds = (b: { date: string; items: UsageItem[] }) => sumSeconds(dayItems(b), pick);
  const weekSeconds = $derived((week?.buckets ?? []).reduce((s, b) => s + daySeconds(b), 0));

  const dateText = (d: string) => {
    const [, m, day] = d.split('-').map(Number);
    return `${m}/${day}`;
  };
  const dayText = (d: string, weekday: number) => `${dateText(d)}（${WEEK_LABELS[weekday]}）`;

  const hourFormat = $derived(new Intl.DateTimeFormat('ja-JP', { timeZone: timezone, hour: 'numeric', hourCycle: 'h23' }));
  const hourOf = (start: number) => Number(hourFormat.formatToParts(start).find((p) => p.type === 'hour')?.value ?? 0);

  const dayBars = $derived<Bar[]>(
    (week?.buckets ?? []).map((b) => ({
      label: WEEK_LABELS[b.weekday],
      value: daySeconds(b),
      title: `${dayText(b.date, b.weekday)} ${duration(daySeconds(b))}`,
      future: b.date > today,
    })),
  );
  const hourBars = $derived<Bar[]>(
    (hours?.buckets ?? []).map((b, i) => {
      const h = hourOf(b.start);
      const v = sumSeconds(b.items, pick);
      return {
        label: i % 6 === 0 ? `${h}時` : '',
        value: v,
        title: `${h}時台 ${duration(v)}`,
        future: b.start > Date.now(),
      };
    }),
  );

  const dayIndex = $derived((week?.buckets ?? []).findIndex((b) => b.date === date));
  const dayBucket = $derived(week?.buckets[dayIndex] ?? null);
  const hourBucket = $derived(mode === 'hour' && pickedHour !== null ? (hours?.buckets[pickedHour] ?? null) : null);

  // 選んでいる日・時間の利用と、その呼び方
  const selected = $derived.by(() => {
    if (hourBucket) {
      const name = `${dateText(hourBucket.date)} ${hourOf(hourBucket.start)}時台`;
      return { items: hourBucket.items, name, title: `${name}の利用` };
    }
    // 履歴がまだ来ていない今日は、状態の値で出す
    const items = dayBucket ? dayItems(dayBucket) : date === today ? status.usage_today : [];
    if (date === today) return { items, name: '今日（いまの時点まで）', title: `今日の利用（${today}）` };
    const name = dayBucket ? dayText(dayBucket.date, dayBucket.weekday) : dateText(date);
    return { items, name, title: `${name}の利用` };
  });
  const selectedSeconds = $derived(sumSeconds(selected.items, pick));

  // 右の合計: 時間を選んでいるときは、その日。それ以外は、その週
  const wide = $derived(
    hourBucket && dayBucket
      ? { name: dayText(dayBucket.date, dayBucket.weekday), seconds: daySeconds(dayBucket) }
      : { name: `今週（${dateText(weekFrom)}〜）`, seconds: weekSeconds },
  );

  function selectDay(i: number) {
    const b = week?.buckets[i];
    if (b) pickedDate = b.date;
    pickedHour = null;
  }

  /** 1 時間ごとのグラフで、見る日を前後にずらす（この週の中だけ） */
  function moveDay(n: number) {
    const next = addDays(date, n);
    if (next < weekFrom || next > today) return;
    pickedDate = next;
    pickedHour = null;
  }

  function setMode(m: ChartMode) {
    usagePrefs.chart = m;
    pickedHour = null;
  }
</script>

<section class="stats">
  <div class="totals">
    <div class="total">
      <span class="total-label">{selected.name}</span>
      <span class="total-value">{duration(selectedSeconds)}</span>
    </div>
    <div class="total">
      <span class="total-label">{wide.name}</span>
      <span class="total-value">{week ? duration(wide.seconds) : '…'}</span>
    </div>
  </div>

  <div class="row chart-head">
    <SegmentedButton
      label="棒グラフの単位"
      value={mode}
      options={[
        { value: 'day', label: '1日ごと' },
        { value: 'hour', label: '1時間ごと' },
      ]}
      onchange={setMode}
    />
    {#if mode === 'hour'}
      <span class="grow"></span>
      <IconButton label="前の日" icon={ChevronLeft} disabled={addDays(date, -1) < weekFrom} onclick={() => moveDay(-1)} />
      <IconButton label="次の日" icon={ChevronRight} disabled={addDays(date, 1) > today} onclick={() => moveDay(1)} />
    {/if}
  </div>

  {#if mode === 'day'}
    <UsageChart bars={dayBars} selected={dayIndex >= 0 ? dayIndex : null} onselect={selectDay} />
  {:else}
    <!-- 1 時間は 60 分までなので、棒の高さの最大は 60 分にそろえる -->
    <UsageChart bars={hourBars} selected={pickedHour} max={3600} onselect={(i) => (pickedHour = pickedHour === i ? null : i)} />
  {/if}

  {@render children?.({ items: selected.items, title: selected.title })}
</section>

<style>
  .stats {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .totals {
    display: flex;
    gap: var(--space-8);
    flex-wrap: wrap;
  }
  .total {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .total-label {
    font-size: var(--fs-base);
    color: var(--text-muted);
  }
  .total-value {
    font-size: var(--fs-2xl);
    font-weight: var(--fw-bold);
  }
  .chart-head {
    min-height: 36px;
  }
  .chart-head :global(.segmented) {
    flex: none;
  }
  .chart-head :global(.segmented-option) {
    white-space: nowrap;
  }
</style>
