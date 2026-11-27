<script lang="ts">
  import { onMount } from 'svelte';
  import { errorText } from '../lib/errors';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { DeviceInfo, UserStatus } from '../lib/types';
  import DeviceList from './DeviceList.svelte';
  import Page from './Page.svelte';

  // 自分のデバイスの一覧。削除すると、友達に通知される
  let { onback, onunlink }: { onback: () => void; onunlink: () => void } = $props();

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let status = $state<UserStatus | null>(null);

  async function refresh() {
    try {
      status = await api.status(saved.userId);
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  onMount(() => {
    void refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });

  async function remove(d: DeviceInfo) {
    // 使っているデバイス自身は、「連携の解除」と同じ（このデバイスの設定も消す）
    if (d.id === saved.deviceId) return onunlink();
    const ok = await ui.confirm({
      title: 'デバイスを削除しますか？',
      body: `「${d.name}」を削除します。友達に通知されます（利用の記録は残ります）。`,
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
</script>

<Page title="デバイス" {onback}>
  {#if status}
    <DeviceList devices={status.devices} currentId={saved.deviceId} onremove={remove} />
    <p class="muted small">削除したデバイスは、もう友達から見えなくなります。友達も、あなたのデバイスを削除できます。</p>
  {:else}
    <p class="empty">読み込み中…</p>
  {/if}
</Page>
