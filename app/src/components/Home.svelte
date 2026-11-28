<script lang="ts">
  import { onMount } from 'svelte';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import User from '@lucide/svelte/icons/user';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import Menu from '@3pino/ui-kit-web/Menu.svelte';
  import MenuItem from '@3pino/ui-kit-web/MenuItem.svelte';
  import { errorText } from '../lib/errors';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Friend, UserStatus } from '../lib/types';
  import type { UsageTarget } from '../lib/usage';
  import { usagePrefs } from '../lib/usagePrefs.svelte';
  import Page from './Page.svelte';
  import UsageDetail from './UsageDetail.svelte';
  import UsagePanel from './UsagePanel.svelte';

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  // 統計を見る人: 自分、または承認済みの友達（見出しの名前を押して切り替える）
  let friends = $state.raw<Friend[]>([]);
  const accepted = $derived(friends.filter((f) => f.state === 'accepted'));
  const viewId = $derived(usagePrefs.viewing?.id ?? saved.userId);
  const mine = $derived(viewId === saved.userId);
  const viewName = $derived(`${usagePrefs.viewing?.name ?? saved.userName} さん`);

  let status = $state<UserStatus | null>(null);

  async function loadFriends() {
    try {
      friends = await api.friends();
      // 見ていた友達がいなくなっていたら（解消された）、自分に戻す
      const v = usagePrefs.viewing;
      if (v && !accepted.some((f) => f.user.id === v.id)) usagePrefs.viewing = null;
    } catch {
      /* 取れなければ、自分だけの表示のまま（通知は、状態の取得が出す） */
    }
  }

  // 見る人を変えたら、前の人の表示は捨てて取り直す
  $effect(() => {
    const id = viewId;
    status = null;
    let alive = true;
    const refresh = async () => {
      try {
        const s = await api.status(id);
        if (alive) status = s;
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

  onMount(() => void loadFriends());

  const ns = $derived(protection.ns);

  // 利用の詳細（アプリ・サイト・ルール）。戻る操作で、ホームに戻る
  let detail = $state<UsageTarget | null>(null);
  $effect(() => {
    if (!detail) return;
    return ui.pushLayer(() => (detail = null));
  });

  // 見る人の選択メニュー。戻る操作で閉じる
  let menuOpen = $state(false);
  $effect(() => {
    if (!menuOpen) return;
    return ui.pushLayer(() => (menuOpen = false));
  });

  function choose(who: { id: string; name: string } | null) {
    menuOpen = false;
    if ((who?.id ?? saved.userId) === viewId) return;
    detail = null;
    usagePrefs.viewing = who;
  }
</script>

{#if detail && status}
  <UsageDetail userId={viewId} ownerName={mine ? 'あなた' : viewName} {status} target={detail} onback={() => (detail = null)} />
{:else}
  <Page title={viewName} {heading}>
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
  <div class="who">
    {#if accepted.length > 0 || !mine}
      <button
        type="button"
        class="who-button"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onclick={() => {
          menuOpen = !menuOpen;
          if (menuOpen) void loadFriends();
        }}
      >
        <h1 class="page-title who-name">{viewName}</h1>
        <ChevronDown size={18} />
      </button>
      {#if menuOpen}
        <Menu class="who-menu" label="統計を見る人" onclose={() => (menuOpen = false)}>
          <MenuItem icon={mine ? Check : User} onclick={() => choose(null)}>{saved.userName} さん（あなた）</MenuItem>
          {#each accepted as f (f.user.id)}
            <MenuItem icon={viewId === f.user.id ? Check : User} onclick={() => choose({ id: f.user.id, name: f.user.display_name })}>{f.user.display_name} さん</MenuItem>
          {/each}
        </Menu>
      {/if}
    {:else}
      <h1 class="page-title">{viewName}</h1>
    {/if}
  </div>
{/snippet}

<style>
  .who {
    position: relative;
    flex: 0 1 auto;
    min-width: 0;
  }
  .who-button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-2) var(--space-3);
    margin-left: calc(var(--space-3) * -1);
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
  }
  .who-button:hover {
    background: var(--hover);
  }
  .who-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who :global(.who-menu) {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: var(--space-2);
    min-width: 220px;
  }
</style>
