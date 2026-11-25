<script lang="ts">
  import { onMount } from 'svelte';
  import Events from './Events.svelte';
  import Friends from './Friends.svelte';
  import Home from './Home.svelte';
  import Setup from './Setup.svelte';
  import { names } from './lib/names.svelte';
  import { session } from './lib/session.svelte';

  type Tab = 'home' | 'friends' | 'events';
  let tab = $state<Tab>('home');

  onMount(() => {
    names.load();
  });

  const TABS: [Tab, string][] = [
    ['home', 'ホーム'],
    ['friends', '友達'],
    ['events', '通知'],
  ];
</script>

{#if !session.saved}
  <Setup />
{:else}
  <div class="shell">
    <main>
      {#if tab === 'home'}<Home />{:else if tab === 'friends'}<Friends />{:else}<Events />{/if}
    </main>
    <nav>
      {#each TABS as [id, label]}
        <button class:active={tab === id} onclick={() => (tab = id)}>{label}</button>
      {/each}
    </nav>
  </div>
{/if}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100dvh;
  }
  main {
    flex: 1;
    overflow-y: auto;
    padding: 16px 16px 24px;
    padding-top: max(16px, env(safe-area-inset-top));
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
  }
  nav {
    display: flex;
    border-top: 1px solid var(--line);
    background: var(--card);
    /* Android のジェスチャーバーに重ならないようにする（env が 0 のときの下限つき） */
    padding-bottom: max(env(safe-area-inset-bottom), 20px);
  }
  nav button {
    flex: 1;
    border: none;
    border-radius: 0;
    padding: 12px 0;
    background: transparent;
    color: var(--muted);
  }
  nav button.active {
    color: var(--accent);
    font-weight: 600;
  }
</style>
