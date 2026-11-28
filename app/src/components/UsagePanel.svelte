<script lang="ts">
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import Toggle from '@3pino/ui-kit-web/Toggle.svelte';
  import { dateTime, duration } from '../lib/format';
  import { hidden } from '../lib/hidden.svelte';
  import { ruleDisplayName } from '../lib/limits';
  import { names } from '../lib/names.svelte';
  import type { UserStatus } from '../lib/types';
  import type { UsageItem } from '../lib/types';
  import { buildRows, homePick, SHORT_SECONDS, type UsageTarget } from '../lib/usage';
  import { usagePrefs } from '../lib/usagePrefs.svelte';
  import KeyIcon from './KeyIcon.svelte';
  import UsageStats from './UsageStats.svelte';

  // 1 人の利用のまとめ: 表示の切り替え・合計・棒グラフ・選んだ日（時間）の利用の一覧（ルールごとにまとめられる）。
  // 行を押すと、詳細の画面（`onopen`）を開く
  let {
    userId,
    status,
    onopen,
  }: {
    userId: string;
    status: UserStatus;
    onopen: (target: UsageTarget) => void;
  } = $props();

  $effect(() => names.learn(status.labels));

  const unlocked = $derived(status.unlock_until > Date.now());

  // 「続きを表示」: 3 分以下の利用（非表示のアプリの表示とは別）
  let showAll = $state(false);
  /** 開いているルールの行（中身を出している） */
  let expanded = $state.raw<Set<string>>(new Set());

  const pick = $derived(homePick(hidden.has, usagePrefs.showHidden));
  const rules = $derived(status.rules.map((r) => r.rule));
  const rowsOf = (all: UsageItem[], ruleSeconds: Record<string, number>) =>
    buildRows(
      all.filter((u) => usagePrefs.showHidden || !hidden.has(u.key)),
      rules,
      usagePrefs.byRule,
      ruleSeconds,
    );

  function toggleExpanded(id: string) {
    const next = new Set(expanded);
    if (!next.delete(id)) next.add(id);
    expanded = next;
  }
</script>

{#if unlocked}
  <div class="card card-warn">制限を一時解除中です（{dateTime(status.unlock_until)} まで）</div>
{/if}

<section class="prefs">
  <SettingRow name="非表示アプリを表示" description="「非表示のアプリ」も、一覧と合計に含めます。">
    {#snippet control()}
      <div class="setting-row-control-slot"><Toggle bind:checked={usagePrefs.showHidden} label="非表示アプリを表示" /></div>
    {/snippet}
  </SettingRow>
  <SettingRow name="カテゴリごとに表示" description="ルールに入っているものを、ルールごとにまとめます。">
    {#snippet control()}
      <div class="setting-row-control-slot"><Toggle bind:checked={usagePrefs.byRule} label="カテゴリごとに表示" /></div>
    {/snippet}
  </SettingRow>
</section>

<UsageStats {userId} {status} {pick}>
  {#snippet children(sel)}
{@const rows = rowsOf(sel.items, sel.rules)}
{@const main = rows.filter((r) => r.seconds > SHORT_SECONDS)}
{@const restCount = rows.length - main.length}
{@const shown = (showAll ? rows : main).slice(0, 60)}
<section class="status-block">
  <h2 class="block-title">{sel.title}</h2>
  {#each shown as row (row.kind === 'rule' ? `rule:${row.rule.id}` : row.key)}
    {#if row.kind === 'key'}
      <button type="button" class="row usage-row" class:limited={row.limited} class:dim={hidden.has(row.key)} onclick={() => onopen({ kind: 'key', key: row.key })}>
        <KeyIcon key={row.key} size={28} />
        <span class="grow usage-name">{names.label(row.key)}</span>
        <span class="muted">{duration(row.seconds)}</span>
      </button>
    {:else}
      {@const open = expanded.has(row.rule.id)}
      <div class="row usage-row rule-row" class:limited={row.limited}>
        <!-- アイコンは、ルール全体の詳細へ。名前は、中身の開閉 -->
        <button type="button" class="rule-icon" aria-label="{ruleDisplayName(row.rule)} の詳細" onclick={() => onopen({ kind: 'rule', id: row.rule.id })}>
          <SlidersHorizontal size={18} />
        </button>
        <button type="button" class="grow row rule-name" aria-expanded={open} onclick={() => toggleExpanded(row.rule.id)}>
          <span class="grow usage-name">{ruleDisplayName(row.rule)}</span>
          <span class="muted">{duration(row.seconds)}</span>
          {#if open}<ChevronDown size={16} />{:else}<ChevronRight size={16} />{/if}
        </button>
      </div>
      {#if open}
        {#each row.children as c (c.key)}
          <button type="button" class="row usage-row child-row" class:limited={row.limited} class:dim={hidden.has(c.key)} onclick={() => onopen({ kind: 'key', key: c.key })}>
            <KeyIcon key={c.key} size={24} />
            <span class="grow usage-name">{names.label(c.key)}</span>
            <span class="muted">{duration(c.seconds)}</span>
          </button>
        {/each}
      {/if}
    {/if}
  {:else}
    <p class="empty">{restCount > 0 ? '3 分を超えて使ったものはありません。' : '記録がありません。'}</p>
  {/each}
  {#if restCount > 0 || showAll}
    <button type="button" class="link-button small" onclick={() => (showAll = !showAll)}>
      {showAll ? '閉じる' : `続きを表示（${restCount} 件）`}
    </button>
  {/if}
</section>
  {/snippet}
</UsageStats>

<style>
  .prefs {
    display: flex;
    flex-direction: column;
  }
  .status-block {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .usage-row {
    min-height: 44px;
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    text-align: left;
  }
  button.usage-row:hover,
  .rule-name:hover {
    background: var(--hover);
  }
  .rule-row {
    padding: 0;
    gap: var(--space-2);
  }
  .rule-icon {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    margin-left: var(--space-3);
    border: none;
    border-radius: 22%;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .rule-name {
    min-height: 44px;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    text-align: left;
  }
  .child-row {
    padding-left: calc(var(--space-3) + 28px + var(--space-4));
  }
  /* 有効なルールの対象は、太字で目立たせる */
  .limited .usage-name {
    font-weight: var(--fw-bold);
  }
  /* 「非表示のアプリ」を表示しているときは、薄い文字の色にする */
  .dim .usage-name {
    color: var(--text-muted);
  }
  .usage-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link-button {
    align-self: flex-start;
    border: none;
    background: transparent;
    color: var(--accent, inherit);
    padding: var(--space-2) 0;
    text-decoration: underline;
  }
</style>
