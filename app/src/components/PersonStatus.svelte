<script lang="ts">
  import { dateTime, duration } from '../lib/format';
  import { hidden } from '../lib/hidden.svelte';
  import { names } from '../lib/names.svelte';
  import type { UserStatus } from '../lib/types';
  import KeyIcon from './KeyIcon.svelte';

  // 1 人の見守りの状態: 一時解除中の表示と、今日の利用（デバイスは、設定・友達の詳細に出す）
  let { status }: { status: UserStatus } = $props();

  /** これ以下（秒）の利用は、初めは隠す */
  const SHORT_SECONDS = 60;

  $effect(() => names.learn(status.labels));

  const unlocked = $derived(status.unlock_until > Date.now());
  let showAll = $state(false);

  const isMain = (u: { key: string; seconds: number }) => u.seconds > SHORT_SECONDS && !hidden.has(u.key);
  const main = $derived(status.usage_today.filter(isMain));
  const restCount = $derived(status.usage_today.length - main.length);
  const shown = $derived((showAll ? status.usage_today : main).slice(0, 60));
</script>

{#if unlocked}
  <div class="card card-warn">制限を一時解除中です（{dateTime(status.unlock_until)} まで）</div>
{/if}

<section class="status-block">
  <h2 class="block-title">今日の利用（{status.date}）</h2>
  {#each shown as u (u.key)}
    <div class="row usage-row">
      <KeyIcon key={u.key} size={28} />
      <div class="grow usage-name">{names.label(u.key)}</div>
      <div class="muted">{duration(u.seconds)}</div>
    </div>
  {:else}
    <p class="empty">{restCount > 0 ? '1 分以上使ったものはありません。' : 'まだ記録がありません。'}</p>
  {/each}
  {#if restCount > 0 || showAll}
    <button type="button" class="link-button small" onclick={() => (showAll = !showAll)}>
      {showAll ? '閉じる' : `続きを表示（${restCount} 件）`}
    </button>
  {/if}
</section>

<style>
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .usage-row {
    min-height: 32px;
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
