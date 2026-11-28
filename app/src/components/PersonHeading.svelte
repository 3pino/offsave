<script lang="ts">
  import { onMount } from 'svelte';
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import User from '@lucide/svelte/icons/user';
  import Menu from '@3pino/ui-kit-web/Menu.svelte';
  import MenuItem from '@3pino/ui-kit-web/MenuItem.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Friend } from '../lib/types';
  import { viewing } from '../lib/viewing.svelte';

  // 見出しの「{名前} さん ▼」。押すと、見る人（自分・承認済みの友達）を切り替えるメニューが出る。ホームとルールで使う
  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let friends = $state.raw<Friend[]>([]);
  const accepted = $derived(friends.filter((f) => f.state === 'accepted'));

  async function loadFriends() {
    try {
      friends = await api.friends();
      // 見ていた友達がいなくなっていたら（解消された）、自分に戻す
      if (!viewing.mine && !accepted.some((f) => f.user.id === viewing.id)) viewing.set(null);
    } catch {
      /* 取れなければ、いまの表示のまま */
    }
  }
  onMount(() => void loadFriends());

  // 戻る操作で閉じる
  let menuOpen = $state(false);
  $effect(() => {
    if (!menuOpen) return;
    return ui.pushLayer(() => (menuOpen = false));
  });

  function choose(who: { id: string; name: string } | null) {
    menuOpen = false;
    viewing.set(who);
  }
</script>

<div class="who">
  {#if accepted.length > 0 || !viewing.mine}
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
      <h1 class="page-title who-name">{viewing.name}</h1>
      <ChevronDown size={18} />
    </button>
    {#if menuOpen}
      <Menu class="who-menu" label="見る人" onclose={() => (menuOpen = false)}>
        <MenuItem icon={viewing.mine ? Check : User} onclick={() => choose(null)}>{saved.userName} さん（あなた）</MenuItem>
        {#each accepted as f (f.user.id)}
          <MenuItem icon={viewing.id === f.user.id ? Check : User} onclick={() => choose({ id: f.user.id, name: f.user.display_name })}>{f.user.display_name} さん</MenuItem>
        {/each}
      </Menu>
    {/if}
  {:else}
    <h1 class="page-title">{viewing.name}</h1>
  {/if}
</div>

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
