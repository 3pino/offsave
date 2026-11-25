<script lang="ts">
  import { untrack } from 'svelte';
  import { isAndroid } from './lib/native';
  import { keyName } from './lib/format';
  import { names } from './lib/names.svelte';
  import type { Limit, Rule, RuleInput, Target, UsageItem } from './lib/types';

  let {
    initial = null,
    usage = [],
    onSave,
    onCancel,
  }: {
    initial?: Rule | null;
    /** 対象の人が今日使ったもの（ここから選べる） */
    usage?: UsageItem[];
    onSave: (rule: RuleInput) => Promise<void>;
    onCancel: () => void;
  } = $props();

  const WEEK = ['月', '火', '水', '木', '金', '土', '日'];

  // 編集の開始時の値だけを使う（あとから props が変わっても、入力中の内容は消さない）
  const init = untrack(() => initial);
  const initLimit = init?.limit;

  let name = $state(init?.name ?? '');
  let enabled = $state(init?.enabled ?? true);
  let targets = $state<Target[]>(init ? init.targets.map((t) => ({ label: t.label, keys: [...t.keys] })) : []);
  let kind = $state<'daily_total' | 'window'>(initLimit?.type ?? 'daily_total');
  let hours = $state(initLimit?.type === 'daily_total' ? Math.floor(initLimit.seconds / 3600) : 1);
  let minutes = $state(initLimit?.type === 'daily_total' ? Math.floor((initLimit.seconds % 3600) / 60) : 0);
  let days = $state(initLimit?.type === 'window' ? initLimit.days : 0x7f);
  let start = $state(initLimit?.type === 'window' ? toTime(initLimit.start_min) : '22:00');
  let end = $state(initLimit?.type === 'window' ? toTime(initLimit.end_min) : '06:00');

  let manualKey = $state('');
  let manualLabel = $state('');
  let search = $state('');
  let picking = $state(false);
  let busy = $state(false);
  let error = $state('');

  function toTime(min: number): string {
    const m = min === 1440 ? 0 : min;
    return `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
  }
  function fromTime(s: string): number {
    const [h, m] = s.split(':').map(Number);
    return h * 60 + m;
  }

  const has = (key: string) => targets.some((t) => t.keys.includes(key));

  function addTarget(label: string, key: string) {
    if (!has(key)) targets.push({ label, keys: [key] });
  }

  function addManual() {
    let key = manualKey.trim();
    if (!key) return;
    // 接頭辞がなければ、ドメインとして扱う
    if (!/^(android|win|linux|domain):/.test(key)) key = `domain:${key}`;
    addTarget(manualLabel.trim() || keyName(key), key);
    manualKey = '';
    manualLabel = '';
  }

  const filtered = $derived(
    names.apps.filter((a) => (a.label + a.package).toLowerCase().includes(search.toLowerCase())).slice(0, 40),
  );

  async function save() {
    error = '';
    if (!name.trim()) return (error = 'ルールの名前を入力してください');
    if (targets.length === 0) return (error = '対象のアプリかサイトを 1 つ以上選んでください');
    let limit: Limit;
    if (kind === 'daily_total') {
      limit = { type: 'daily_total', seconds: Math.max(0, hours) * 3600 + Math.max(0, minutes) * 60 };
    } else {
      const s = fromTime(start);
      let e = fromTime(end);
      if (e === 0) e = 1440;
      if (s === e) return (error = '開始と終了が同じです');
      if ((days & 0x7f) === 0) return (error = '曜日を選んでください');
      limit = { type: 'window', days, start_min: s, end_min: e };
    }
    busy = true;
    try {
      await onSave({ name: name.trim(), targets, limit, enabled });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="card">
  <h2 style="margin-top: 0">{init ? 'ルールを編集' : 'ルールを追加'}</h2>

  <label for="rn">ルールの名前</label>
  <input id="rn" placeholder="例: ゲーム" bind:value={name} />

  <h3>対象（合計で数えます）</h3>
  {#each targets as t, i}
    <div class="row" style="margin-bottom: 4px">
      <div class="grow">
        <b>{t.label}</b>
        <span class="muted">{t.keys.map(keyName).join(', ')}</span>
      </div>
      <button class="small" onclick={() => targets.splice(i, 1)}>外す</button>
    </div>
  {:else}
    <p class="muted">まだありません。</p>
  {/each}

  {#if usage.length > 0}
    <div class="muted">今日使ったもの（押すと追加）</div>
    {#each usage as u}
      <span class="chip" role="button" tabindex="0" onclick={() => addTarget(names.label(u.key), u.key)} onkeydown={() => {}}>
        {has(u.key) ? '✓ ' : '＋ '}{names.label(u.key)}
      </span>
    {/each}
  {/if}

  {#if isAndroid}
    <div style="margin-top: 8px">
      <button class="small" onclick={() => (picking = !picking)}>このデバイスのアプリから選ぶ</button>
    </div>
    {#if picking}
      <input placeholder="検索" bind:value={search} style="margin-top: 6px" />
      <div style="max-height: 200px; overflow: auto; margin-top: 4px">
        {#each filtered as a}
          <div class="row" style="padding: 3px 0">
            <div class="grow">{a.label} <span class="muted">{a.package}</span></div>
            <button class="small" disabled={has(`android:${a.package}`)} onclick={() => addTarget(a.label, `android:${a.package}`)}>
              {has(`android:${a.package}`) ? '追加済み' : '追加'}
            </button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <h3>サイトや別の OS のアプリを追加</h3>
  <div class="row wrap">
    <div class="grow"><input placeholder="youtube.com / android:com.x / win:game.exe" bind:value={manualKey} autocapitalize="off" /></div>
    <div style="width: 130px"><input placeholder="名前（省略可）" bind:value={manualLabel} /></div>
    <button class="small" onclick={addManual}>追加</button>
  </div>

  <h3>制限</h3>
  <div class="row">
    <label class="row" style="margin: 0; color: var(--fg)"><input type="radio" value="daily_total" bind:group={kind} /> 1日の合計</label>
    <label class="row" style="margin: 0; color: var(--fg)"><input type="radio" value="window" bind:group={kind} /> 使えない時間帯</label>
  </div>

  {#if kind === 'daily_total'}
    <div class="row" style="margin-top: 8px">
      <input type="number" min="0" max="24" bind:value={hours} style="width: 80px" /> 時間
      <input type="number" min="0" max="59" bind:value={minutes} style="width: 80px" /> 分まで
    </div>
  {:else}
    <div class="row wrap" style="margin-top: 8px">
      {#each WEEK as w, i}
        <label class="row" style="margin: 0; color: var(--fg)">
          <input type="checkbox" checked={!!(days & (1 << i))} onchange={() => (days ^= 1 << i)} />{w}
        </label>
      {/each}
    </div>
    <div class="row" style="margin-top: 8px">
      <input type="time" bind:value={start} style="width: 130px" /> から
      <input type="time" bind:value={end} style="width: 130px" /> まで使えない
    </div>
    <p class="muted">日をまたぐ指定もできます（例: 22:00 から 06:00）。曜日は開始する日です。</p>
  {/if}

  <label class="row" style="margin-top: 12px; color: var(--fg)"><input type="checkbox" bind:checked={enabled} /> 有効にする</label>

  {#if error}<div class="error">{error}</div>{/if}
  <div class="row" style="margin-top: 12px">
    <button class="primary" disabled={busy} onclick={save}>{busy ? '保存中…' : '保存'}</button>
    <button onclick={onCancel}>キャンセル</button>
  </div>
</div>
