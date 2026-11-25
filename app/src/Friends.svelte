<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from './lib/session.svelte';
  import FriendView from './FriendView.svelte';
  import type { Friend } from './lib/types';

  const api = $derived(session.api!);

  let friends = $state<Friend[]>([]);
  let error = $state('');
  let login = $state('');
  let open = $state<Friend | null>(null);

  async function refresh() {
    try {
      friends = await api.friends();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });

  async function act(f: () => Promise<unknown>) {
    try {
      await f();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    await refresh();
  }

  const request = () =>
    act(async () => {
      await api.requestFriend(login.trim());
      login = '';
    });
</script>

{#if open}
  <FriendView friend={open} onBack={() => { open = null; refresh(); }} />
{:else}
  <h1>友達</h1>
  {#if error}<div class="error">{error}</div>{/if}

  {#each friends as f}
    <div class="card">
      <div class="row">
        <div class="grow">
          <b>{f.user.display_name}</b> <span class="muted">{f.user.login_name}</span>
        </div>
        {#if f.state === 'accepted'}
          <button class="small primary" onclick={() => (open = f)}>見る</button>
        {:else if f.state === 'pending_in'}
          <button class="small primary" onclick={() => act(() => api.acceptFriend(f.user.id))}>承認</button>
          <button class="small" onclick={() => act(() => api.removeFriend(f.user.id))}>断る</button>
        {:else}
          <span class="muted">承認待ち</span>
          <button class="small" onclick={() => act(() => api.removeFriend(f.user.id))}>取り消す</button>
        {/if}
      </div>
      {#if f.state === 'pending_in'}
        <div class="muted">承認すると、お互いの利用状況が見え、お互いにルールを決められるようになります。</div>
      {/if}
    </div>
  {:else}
    <p class="muted">まだ友達がいません。</p>
  {/each}

  <h2>友達を追加</h2>
  <div class="card">
    <label for="login">相手の Tailscale のログイン名</label>
    <div class="row">
      <div class="grow"><input id="login" placeholder="bob@example.com" bind:value={login} autocapitalize="off" /></div>
      <button class="primary" disabled={!login.trim()} onclick={request}>申請</button>
    </div>
    <p class="muted">相手が先に offsave を開いて接続している必要があります。</p>
  </div>
{/if}
