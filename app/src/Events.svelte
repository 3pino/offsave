<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from './lib/session.svelte';
  import { ALERT_KINDS, EVENT_LABEL, dateTime } from './lib/format';
  import type { OffsaveEvent } from './lib/types';

  const api = $derived(session.api!);
  const me = $derived(session.saved!.userId);

  let events = $state<OffsaveEvent[]>([]);
  let error = $state('');

  async function refresh() {
    try {
      events = await api.events(100);
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });
</script>

<h1>通知</h1>
{#if error}<div class="error">{error}</div>{/if}

{#each events as e (e.id)}
  <div class="card">
    <div class="row">
      <b class="grow {ALERT_KINDS.includes(e.kind) ? 'bad' : ''}">
        {e.user.id === me ? '自分' : e.user.display_name}: {EVENT_LABEL[e.kind]}
      </b>
      <span class="muted">{dateTime(e.at)}</span>
    </div>
    <div style="white-space: pre-wrap">{e.message}</div>
    {#if e.device_name}<div class="muted">{e.device_name}</div>{/if}
  </div>
{:else}
  <p class="muted">まだ通知はありません。</p>
{/each}
