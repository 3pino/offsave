<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { errorText } from '../lib/errors';
  import { hidden } from '../lib/hidden.svelte';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { UserStatus } from '../lib/types';
  import type { UsageTarget } from '../lib/usage';
  import { viewing } from '../lib/viewing.svelte';
  import Page from './Page.svelte';
  import PersonHeading from './PersonHeading.svelte';
  import UsageDetail from './UsageDetail.svelte';
  import UsagePanel from './UsagePanel.svelte';

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  // 統計を見る人: 自分、または承認済みの友達（見出しの名前を押して切り替える）
  const viewId = $derived(viewing.id);
  const mine = $derived(viewing.mine);

  let status = $state<UserStatus | null>(null);

  // 見る人を変えたら、前の人の表示は捨てて取り直す
  $effect(() => {
    const id = viewId;
    status = null;
    let alive = true;
    const refresh = async () => {
      try {
        const s = await api.status(id);
        if (alive) {
          hidden.sync(id, s.hidden);
          status = s;
        }
      } catch (e) {
        if (alive) ui.toast(errorText(e), 'error');
      }
    };
    void refresh();
    const t = setInterval(refresh, 3000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  });

  onMount(() => {
    // 以前はこのデバイスだけに持っていた非表示の一覧を、自分の一覧に和集合で引き取る
    void hidden.migrateLegacy(api, saved.userId);
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
  <UsageDetail userId={viewId} ownerName={mine ? 'あなた' : viewing.name} {status} target={detail} onback={() => (detail = null)} />
{:else}
  <Page title={viewing.name} {heading}>
    {#if mine && ns && protection.incomplete}
      <div class="card card-warn">
        <div>このデバイスの監視が、まだ完全ではありません。</div>
        <div><Button variant="primary" onclick={() => ui.openSettings('protection')}>設定を開く</Button></div>
      </div>
    {:else if mine && ns && !ns.connected && ns.lastError}
      <div class="card card-warn">
        <div>{ns.lastError}</div>
        <div class="muted small">制限は、最後に受け取った内容で続きます。</div>
      </div>
    {/if}

    {#if status}
      {#key viewId}
        <UsagePanel userId={viewId} {status} onopen={(t) => (detail = t)} />
      {/key}
    {/if}
  </Page>
{/if}

{#snippet heading()}
  <PersonHeading />
{/snippet}
