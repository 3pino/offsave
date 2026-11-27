<script lang="ts">
  import { onMount } from 'svelte';
  import { events } from '../lib/events.svelte';
  import { ALERT_KINDS, EVENT_LABEL, dateTime } from '../lib/format';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import Page from './Page.svelte';

  const me = $derived(session.saved!.userId);

  // この画面を見ている間は、見たものとして数える（ナビゲーションの点を消す）
  $effect(() => {
    void events.items;
    events.markSeen();
  });

  onMount(() => {
    void events.refresh();
  });

  $effect(() => {
    if (events.error) ui.toast(events.error, 'error');
  });
</script>

<Page title="通知" header={false}>
  {#each events.items as e (e.id)}
    <div class="card">
      <div class="card-title">
        <span class="grow" class:alert={ALERT_KINDS.includes(e.kind)}>
          {e.user.id === me ? '自分' : e.user.display_name}: {EVENT_LABEL[e.kind]}
        </span>
        <span class="muted small">{dateTime(e.at)}</span>
      </div>
      <div class="message">{e.message}</div>
      {#if e.device_name}<div class="muted small">{e.device_name}</div>{/if}
    </div>
  {:else}
    <p class="empty">まだ通知はありません。</p>
  {/each}
</Page>

<style>
  .alert {
    color: var(--danger);
  }
  .message {
    white-space: pre-wrap;
  }
</style>
