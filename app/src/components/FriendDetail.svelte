<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { errorText } from '../lib/errors';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import { viewing } from '../lib/viewing.svelte';
  import type { DeviceInfo, Friend, UserStatus } from '../lib/types';
  import DeviceList from './DeviceList.svelte';
  import Page from './Page.svelte';

  // 友達の詳細: デバイスの一覧（削除できる）と、友達の解消。利用の統計はホーム、ルールはルールの見出しで人を選んで見る
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
    const t = setInterval(refresh, 5000);
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
      viewing.drop(friend.user.id);
      onback();
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }
</script>

<Page title="{friend.user.display_name} さん" {onback}>
  {#if status}
    <section class="detail-block">
      <h2 class="block-title">デバイス</h2>
      <DeviceList devices={status.devices} onremove={removeDevice} />
    </section>
  {/if}
  <div><Button variant="danger" onclick={dissolve}>友達を解消する</Button></div>
</Page>

<style>
  .detail-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
</style>
