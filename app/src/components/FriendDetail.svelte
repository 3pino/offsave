<script lang="ts">
  import { onMount } from 'svelte';
  import ListChecks from '@lucide/svelte/icons/list-checks';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { errorText } from '../lib/errors';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { DeviceInfo, Friend, UserStatus } from '../lib/types';
  import type { UsageTarget } from '../lib/usage';
  import DeviceList from './DeviceList.svelte';
  import Page from './Page.svelte';
  import UsageDetail from './UsageDetail.svelte';
  import UsagePanel from './UsagePanel.svelte';

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

  async function removeDevice(d: DeviceInfo) {
    const ok = await ui.confirm({
      title: 'デバイスを削除しますか？',
      body: `${friend.user.display_name} さんの「${d.name}」を、見守りの対象から外します（利用の記録は残ります）。本人に通知されます。このデバイスは、最後に受け取ったルールで制限を続けますが、新しいルールは届きません。`,
      okLabel: '削除する',
      danger: true,
    });
    if (!ok) return;
    try {
      await api.removeDevice(d.id);
      await refresh();
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

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

  // 利用の詳細（アプリ・サイト・ルール）。戻る操作で、友達の画面に戻る
  let detail = $state<UsageTarget | null>(null);
  $effect(() => {
    if (!detail) return;
    return ui.pushLayer(() => (detail = null));
  });
</script>

{#if detail && status}
  <UsageDetail userId={friend.user.id} ownerName="{friend.user.display_name} さん" {status} target={detail} onback={() => (detail = null)} />
{:else}
<Page title="{friend.user.display_name} さん" {onback}>
  <div><Button icon={ListChecks} onclick={() => ui.openRulesOf(friend.user.id)}>この人のルールを見る・変える</Button></div>
  {#if status}
    <section class="detail-block">
      <h2 class="block-title">デバイス</h2>
      <DeviceList devices={status.devices} onremove={removeDevice} />
    </section>
    <UsagePanel userId={friend.user.id} {status} onopen={(t) => (detail = t)} />
  {/if}
  <div><Button variant="danger" onclick={dissolve}>友達を解消する</Button></div>
</Page>
{/if}

<style>
  .detail-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
</style>
