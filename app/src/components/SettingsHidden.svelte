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

  // 「今日の利用」に出さないアプリ・サイト（表示だけの設定。記録や制限には関係しない）
  let { onback }: { onback: () => void } = $props();

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let picking = $state(false);
  let status = $state.raw<UserStatus | null>(null);

  onMount(() => {
    void api.status(saved.userId).then((s) => (status = s)).catch(() => {});
  });

  // 選べるもの: このデバイスのアプリ + 自分が使ったことのあるもの
  const candidates = $derived.by(() => {
    const map = new Map<string, string>();
    for (const a of names.apps) map.set(`android:${a.package}`, a.label);
    for (const l of status?.labels ?? []) if (!map.has(l.key)) map.set(l.key, l.label);
    for (const u of status?.usage_today ?? []) if (!map.has(u.key)) map.set(u.key, names.label(u.key));
    return [...map].map(([key, label]) => ({ key, label }));
  });
</script>

<Page title="今日の利用の表示" {onback}>
  <p class="muted small hidden-note">
    ホーム画面のアプリ・offsave 自身のように、見ても意味のないものを、「今日の利用」に出さないようにします。記録と制限には影響しません。この設定は、このデバイスの画面だけに効きます。
  </p>

  <section class="hidden-block">
    <h2 class="block-title">出さないもの</h2>
    {#each hidden.keys as key (key)}
      <div class="row hidden-row">
        <KeyIcon {key} size={28} />
        <div class="grow hidden-name">{names.label(key)}</div>
        <button type="button" class="hidden-x" aria-label="{names.label(key)} を出すようにする" onclick={() => hidden.remove(key)}><X size={16} /></button>
      </div>
    {:else}
      <p class="empty">ありません。</p>
    {/each}
    <div class="row row-wrap">
      <Button icon={Plus} onclick={() => (picking = true)}>追加</Button>
      <Button onclick={() => hidden.reset()}>初期設定に戻す</Button>
    </div>
  </section>
</Page>

{#if picking}
  <TargetPicker
    title="出さないものを追加"
    {candidates}
    picked={hidden.has}
    onpick={(key) => hidden.add(key)}
    onunpick={(key) => hidden.remove(key)}
    onclose={() => (picking = false)}
  />
{/if}

<style>
  .hidden-note {
    margin: 0;
  }
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
