<script lang="ts">
  import { dateTime, duration } from '../lib/format';
  import { hidden } from '../lib/hidden.svelte';
  import { names } from '../lib/names.svelte';
  import type { UserStatus } from '../lib/types';
  import KeyIcon from './KeyIcon.svelte';

  // 1 人の見守りの状態: 一時解除中の表示と、今日の利用（デバイスは、設定・友達の詳細に出す）
  let { status }: { status: UserStatus } = $props();

  $effect(() => names.learn(status.labels));

  const unlocked = $derived(status.unlock_until > Date.now());
  let showHidden = $state(false);

  const hiddenCount = $derived(status.usage_today.filter((u) => hidden.has(u.key)).length);
  const shown = $derived(
    (showHidden ? status.usage_today : status.usage_today.filter((u) => !hidden.has(u.key))).slice(0, 30),
  );
</script>

{#if unlocked}
  <div class="card card-warn">制限を一時解除中です（{dateTime(status.unlock_until)} まで）</div>
{/if}

<section class="status-block">
  <h2 class="block-title">今日の利用（{status.date}）</h2>
  <div class="card">
    {#each shown as u (u.key)}
      <div class="row usage-row">
        <KeyIcon key={u.key} size={28} />
        <div class="grow usage-name">{names.label(u.key)}</div>
        <div class="muted">{duration(u.seconds)}</div>
      </div>
    {:else}
      <span class="muted">{hiddenCount > 0 ? '表示するものがありません。' : 'まだ記録がありません。'}</span>
    {/each}
    {#if hiddenCount > 0}
      <button type="button" class="link-button small" onclick={() => (showHidden = !showHidden)}>
        {showHidden ? '非表示のものを隠す' : `非表示の ${hiddenCount} 件も見る`}
      </button>
    {/if}
  </div>
</section>

<style>
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
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
