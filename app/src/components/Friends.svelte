<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import { errorText } from '../lib/errors';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Friend } from '../lib/types';
  import FriendDetail from './FriendDetail.svelte';
  import Page from './Page.svelte';

  const api = $derived(session.api!);

  let friends = $state<Friend[]>([]);
  let login = $state('');
  let open = $state<Friend | null>(null);

  // 戻る操作（Android の戻る）で、友達の詳細を閉じる
  $effect(() => {
    if (!open) return;
    return ui.pushLayer(() => (open = null));
  });

  async function refresh() {
    try {
      friends = await api.friends();
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  onMount(() => {
    void refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });

  async function act(f: () => Promise<unknown>) {
    try {
      await f();
    } catch (e) {
      ui.toast(errorText(e), 'error');
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
  <FriendDetail
    friend={open}
    onback={() => {
      open = null;
      void refresh();
    }}
  />
{:else}
  <Page title="友達">
    <section class="friends-block">
      {#each friends as f (f.user.id)}
        <div class="card">
          <div class="card-title">
            <span class="grow">{f.user.display_name} <span class="muted small">{f.user.login_name}</span></span>
            {#if f.state === 'accepted'}
              <Button variant="primary" onclick={() => (open = f)}>見る</Button>
            {:else if f.state === 'pending_in'}
              <Button variant="primary" onclick={() => act(() => api.acceptFriend(f.user.id))}>承認</Button>
              <Button onclick={() => act(() => api.removeFriend(f.user.id))}>断る</Button>
            {:else}
              <span class="muted small">承認待ち</span>
              <Button onclick={() => act(() => api.removeFriend(f.user.id))}>取り消す</Button>
            {/if}
          </div>
          {#if f.state === 'pending_in'}
            <div class="muted small">承認すると、お互いの利用状況が見え、お互いにルールを決められるようになります。</div>
          {/if}
        </div>
      {:else}
        <p class="empty">まだ友達がいません。</p>
      {/each}
    </section>

    <section class="friends-block">
      <h2 class="block-title">友達を追加</h2>
      <div class="card">
        <label class="field">
          <span class="field-label">相手の Tailscale のログイン名</span>
          <div class="row">
            <div class="grow"><TextInput bind:value={login} placeholder="bob@example.com" autocapitalize="off" /></div>
            <Button variant="primary" disabled={!login.trim()} onclick={request}>申請</Button>
          </div>
        </label>
        <p class="muted small">相手が先に offsave を開いて接続している必要があります。</p>
      </div>
    </section>
  </Page>
{/if}

<style>
  .friends-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
</style>
