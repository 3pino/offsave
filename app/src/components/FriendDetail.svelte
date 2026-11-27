<script lang="ts">
  import { onMount } from 'svelte';
  import ListChecks from '@lucide/svelte/icons/list-checks';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { errorText } from '../lib/errors';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Friend, UserStatus } from '../lib/types';
  import Page from './Page.svelte';
  import PersonStatus from './PersonStatus.svelte';

  let { friend, onback }: { friend: Friend; onback: () => void } = $props();

  const api = $derived(session.api!);

  let status = $state<UserStatus | null>(null);

  async function refresh() {
    try {
      status = await api.status(friend.user.id);
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  onMount(() => {
    void refresh();
    const t = setInterval(refresh, 3000);
    return () => clearInterval(t);
  });

  async function dissolve() {
    const ok = await ui.confirm({
      title: '友達を解消しますか？',
      body: `${friend.user.display_name} さんとの友達を解消すると、お互いのルールと見守りが解除され、相手に通知されます。`,
      okLabel: '解消する',
      danger: true,
    });
    if (!ok) return;
    try {
      await api.removeFriend(friend.user.id);
      onback();
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }
</script>

<Page title="{friend.user.display_name} さん" {onback}>
  <div><Button icon={ListChecks} onclick={() => ui.openRulesOf(friend.user.id)}>この人のルールを見る・変える</Button></div>
  {#if status}
    <PersonStatus {status} />
  {/if}
  <div><Button variant="danger" onclick={dissolve}>友達を解消する</Button></div>
</Page>
