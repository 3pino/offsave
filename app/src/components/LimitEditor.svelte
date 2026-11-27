<script lang="ts">
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import SegmentedButton from '@3pino/ui-kit-web/SegmentedButton.svelte';
  import { clock } from '../lib/format';
  import { WEEK_LABELS, newLimit, parseTime } from '../lib/limits';
  import type { Limit } from '../lib/types';

  // 制限 1 つの編集（1 日の合計 / 使えない時間帯 と、曜日）。値は持たず、変えるたびに onchange へ新しい制限を返す
  let {
    limit,
    onchange,
    onremove,
  }: {
    limit: Limit;
    onchange: (l: Limit) => void;
    /** 渡すと、削除ボタンを出す */
    onremove?: () => void;
  } = $props();

  const KINDS = [
    { value: 'daily_total', label: '1日の合計' },
    { value: 'window', label: '使えない時間帯' },
  ] as const;

  const hours = $derived(limit.type === 'daily_total' ? Math.floor(limit.seconds / 3600) : 0);
  const minutes = $derived(limit.type === 'daily_total' ? Math.floor((limit.seconds % 3600) / 60) : 0);

  function setKind(type: Limit['type']) {
    if (type === limit.type) return;
    // 曜日の指定は引き継ぐ
    onchange({ ...newLimit(type), days: limit.days });
  }

  function toggleDay(i: number) {
    onchange({ ...limit, days: limit.days ^ (1 << i) });
  }

  const num = (s: string) => (Number.isFinite(Number(s)) ? Math.max(0, Math.floor(Number(s))) : 0);

  function setTotal(h: number, m: number) {
    if (limit.type === 'daily_total') onchange({ ...limit, seconds: h * 3600 + m * 60 });
  }

  function setStart(s: string) {
    const v = parseTime(s);
    if (limit.type === 'window' && v !== null && v < 1440) onchange({ ...limit, start_min: v });
  }

  function setEnd(s: string) {
    let v = parseTime(s);
    if (limit.type !== 'window' || v === null) return;
    // 0:00 を終わりにしたときは、その日の終わり（24:00）
    if (v === 0) v = 1440;
    onchange({ ...limit, end_min: v });
  }
</script>

<div class="card limit">
  <div class="row">
    <div class="grow">
      <SegmentedButton label="制限の種類" options={[...KINDS]} value={limit.type} onchange={setKind} />
    </div>
    {#if onremove}<IconButton label="この制限を削除" icon={Trash2} onclick={onremove} />{/if}
  </div>

  <div class="field">
    <span class="field-label">{limit.type === 'window' ? '曜日（始まる日）' : '曜日'}</span>
    <div class="chips" role="group" aria-label="曜日">
      {#each WEEK_LABELS as w, i}
        <button type="button" class="chip" class:chip-on={!!(limit.days & (1 << i))} aria-pressed={!!(limit.days & (1 << i))} onclick={() => toggleDay(i)}>{w}</button>
      {/each}
    </div>
  </div>

  {#if limit.type === 'daily_total'}
    <div class="row">
      <input class="input num" type="number" min="0" max="24" inputmode="numeric" aria-label="時間" value={hours} oninput={(e) => setTotal(num(e.currentTarget.value), minutes)} />
      <span>時間</span>
      <input class="input num" type="number" min="0" max="59" inputmode="numeric" aria-label="分" value={minutes} oninput={(e) => setTotal(hours, num(e.currentTarget.value))} />
      <span>分まで</span>
    </div>
    <p class="muted small">1 日の始まり（既定は 4:00）から数えた、その日の合計です。</p>
  {:else}
    <div class="row row-wrap">
      <input class="input time" type="time" aria-label="開始" value={clock(limit.start_min)} onchange={(e) => setStart(e.currentTarget.value)} />
      <span>から</span>
      <input class="input time" type="time" aria-label="終了" value={clock(limit.end_min)} onchange={(e) => setEnd(e.currentTarget.value)} />
      <span>まで使えない</span>
    </div>
    <p class="muted small">日をまたぐ指定もできます。</p>
  {/if}
</div>

<style>
  .limit {
    background: var(--bg);
  }
  .limit p {
    margin: 0;
  }
  .num {
    width: 80px;
  }
  .time {
    width: 130px;
  }
</style>
