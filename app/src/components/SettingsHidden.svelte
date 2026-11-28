<script lang="ts">
  import { onMount } from 'svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { hidden } from '../lib/hidden.svelte';
  import { names } from '../lib/names.svelte';
  import { session } from '../lib/session.svelte';
  import type { UserStatus } from '../lib/types';
  import KeyIcon from './KeyIcon.svelte';
  import Page from './Page.svelte';
  import TargetPicker from './TargetPicker.svelte';

  // 「今日の利用」で初めは隠すアプリ・サイト（表示だけの設定。記録や制限には関係しない。友達も同じ一覧を見る）
  let { onback }: { onback: () => void } = $props();

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let picking = $state(false);
  let status = $state.raw<UserStatus | null>(null);

  // 一覧は、自分のものをサーバーから取って出す（友達も同じ一覧を見る）
  onMount(() => {
    void api
      .status(saved.userId)
      .then((s) => {
        hidden.sync(saved.userId, s.hidden);
        status = s;
      })
      .catch(() => {});
  });
  const change = (add: string[], remove: string[]) => void hidden.change(api, saved.userId, add, remove);

  // 選べるもの: このデバイスのアプリ + 自分が使ったことのあるもの
  const candidates = $derived.by(() => {
    const map = new Map<string, string>();
    for (const a of names.apps) map.set(`android:${a.package}`, a.label);
    for (const l of status?.labels ?? []) if (!map.has(l.key)) map.set(l.key, l.label);
    for (const u of status?.usage_today ?? []) if (!map.has(u.key)) map.set(u.key, names.label(u.key));
    return [...map].map(([key, label]) => ({ key, label }));
  });
</script>

<Page title="非表示のアプリ" {onback}>
  <section class="hidden-block">
    {#each status ? hidden.keys : [] as key (key)}
      <div class="row hidden-row">
        <KeyIcon {key} size={28} />
        <div class="grow hidden-name">{names.label(key)}</div>
        <button type="button" class="hidden-x" aria-label="{names.label(key)} を出すようにする" onclick={() => change([], [key])}><X size={16} /></button>
      </div>
    {:else}
      <p class="empty">まだありません。</p>
    {/each}
    <div class="row row-wrap">
      <Button icon={Plus} onclick={() => (picking = true)}>追加</Button>
    </div>
  </section>
</Page>

{#if picking}
  <TargetPicker
    title="非表示にするアプリ"
    {candidates}
    picked={hidden.has}
    onpick={(key) => change([key], [])}
    onunpick={(key) => change([], [key])}
    onclose={() => (picking = false)}
  />
{/if}

<style>
  .hidden-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .hidden-row {
    padding: var(--space-2) 0;
  }
  .hidden-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hidden-x {
    display: inline-flex;
    border: none;
    background: transparent;
    color: inherit;
    padding: var(--space-2);
    border-radius: var(--radius-pill);
  }
</style>
