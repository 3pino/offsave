<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { errorText } from '../lib/errors';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { UserStatus } from '../lib/types';
  import type { UsageTarget } from '../lib/usage';
  import Page from './Page.svelte';
  import UsageDetail from './UsageDetail.svelte';
  import UsagePanel from './UsagePanel.svelte';

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
    const t = setInterval(refresh, 3000);
    return () => clearInterval(t);
  });

  const ns = $derived(protection.ns);

  // 利用の詳細（アプリ・サイト・ルール）。戻る操作で、ホームに戻る
  let detail = $state<UsageTarget | null>(null);
  $effect(() => {
    if (!detail) return;
    return ui.pushLayer(() => (detail = null));
  });
</script>

{#if detail && status}
  <UsageDetail userId={saved.userId} ownerName="あなた" {status} target={detail} onback={() => (detail = null)} />
{:else}
<Page title="{saved.userName} さん" header={false}>
  {#if ns && protection.incomplete}
    <div class="card card-warn">
      <div>このデバイスの監視が、まだ完全ではありません。</div>
      <div><Button variant="primary" onclick={() => ui.openSettings('protection')}>設定を開く</Button></div>
    </div>
  {:else if ns && !ns.connected && ns.lastError}
    <div class="card card-warn">
      <div>{ns.lastError}</div>
      <div class="muted small">制限は、最後に受け取った内容で続きます。</div>
    </div>
  {/if}

  {#if status}
    <UsagePanel userId={saved.userId} {status} onopen={(t) => (detail = t)} />
  {/if}
</Page>
{/if}
