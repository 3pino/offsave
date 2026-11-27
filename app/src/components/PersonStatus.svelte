<script lang="ts">
  import { ago, dateTime, duration } from '../lib/format';
  import { names } from '../lib/names.svelte';
  import type { UserStatus } from '../lib/types';

  // 1 人の見守りの状態: 一時解除中の表示・デバイス・今日の利用
  let { status }: { status: UserStatus } = $props();

  const STATE: Record<string, [string, string]> = {
    online: ['接続中', 'pill-ok'],
    suspect: ['連絡なし', 'pill-bad'],
    offline: ['停止', 'pill-warn'],
    new: ['未接続', 'pill-muted'],
  };

  $effect(() => names.learn(status.labels));

  const unlocked = $derived(status.unlock_until > Date.now());
</script>

{#if unlocked}
  <div class="card card-warn">制限を一時解除中です（{dateTime(status.unlock_until)} まで）</div>
{/if}

<section class="status-block">
  <h2 class="block-title">デバイス</h2>
  {#each status.devices as d (d.id)}
    {@const s = STATE[d.state] ?? [d.state, 'pill-muted']}
    <div class="card">
      <div class="card-title">
        <span class="grow">{d.name}</span>
        <span class="pill {s[1]}">{s[0]}</span>
      </div>
      <div class="muted small">
        {d.platform} ・ 最後の連絡 {d.last_seen ? ago(d.last_seen) : 'なし'}
        {#if d.foreground}・ いま {names.label(d.foreground)}{/if}
      </div>
      {#if d.state !== 'new' && !(d.flags.accessibility && d.flags.device_admin)}
        <div class="error-text small">
          保護が不完全です:
          {#if !d.flags.accessibility}Accessibility が無効 {/if}
          {#if !d.flags.device_admin}端末管理者が無効{/if}
        </div>
      {/if}
    </div>
  {:else}
    <p class="empty">デバイスがありません。</p>
  {/each}
</section>

<section class="status-block">
  <h2 class="block-title">今日の利用（{status.date}）</h2>
  <div class="card">
    {#each status.usage_today.slice(0, 12) as u (u.key)}
      <div class="row">
        <div class="grow">{names.label(u.key)}</div>
        <div class="muted">{duration(u.seconds)}</div>
      </div>
    {:else}
      <span class="muted">まだ記録がありません。</span>
    {/each}
  </div>
</section>

<style>
  .status-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
</style>
