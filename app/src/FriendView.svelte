<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from './lib/session.svelte';
  import RuleEditor from './RuleEditor.svelte';
  import StatusView from './StatusView.svelte';
  import type { Friend, Rule, RuleInput, UserStatus } from './lib/types';

  let { friend, onBack }: { friend: Friend; onBack: () => void } = $props();

  const api = $derived(session.api!);

  let status = $state<UserStatus | null>(null);
  let error = $state('');
  let editing = $state<Rule | null>(null);
  let adding = $state(false);

  async function refresh() {
    try {
      status = await api.status(friend.user.id);
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    refresh();
    const t = setInterval(refresh, 3000);
    return () => clearInterval(t);
  });

  async function save(input: RuleInput) {
    if (editing) await api.updateRule(editing.id, input);
    else await api.createRule(friend.user.id, input);
    editing = null;
    adding = false;
    await refresh();
  }

  async function act(f: () => Promise<unknown>) {
    try {
      await f();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    await refresh();
  }

  async function dissolve() {
    if (!confirm(`${friend.user.display_name} さんとの友達を解消しますか？\nお互いのルールと見守りが解除され、相手に通知されます。`)) return;
    try {
      await api.removeFriend(friend.user.id);
      onBack();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="row" style="margin-bottom: 8px">
  <button class="small" onclick={onBack}>← 戻る</button>
  <h1 style="margin: 0">{friend.user.display_name} さん</h1>
</div>
{#if error}<div class="error">{error}</div>{/if}

{#if status}
  {#if adding || editing}
    <RuleEditor
      initial={editing}
      usage={status.usage_today}
      onSave={save}
      onCancel={() => {
        adding = false;
        editing = null;
      }}
    />
  {:else}
    <StatusView
      {status}
      mine={false}
      onEdit={(r) => (editing = r)}
      onDelete={(r) => confirm(`ルール「${r.name}」を削除しますか？`) && act(() => api.deleteRule(r.id))}
      onOverride={(date, ruleId) => act(() => api.createOverride(friend.user.id, date, ruleId))}
      onDeleteOverride={(id) => act(() => api.deleteOverride(id))}
    />
    <div style="margin-top: 12px"><button class="primary" onclick={() => (adding = true)}>ルールを追加</button></div>
    <div style="margin-top: 32px"><button class="danger" onclick={dissolve}>友達を解消する</button></div>
  {/if}
{/if}
