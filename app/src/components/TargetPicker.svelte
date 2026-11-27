<script lang="ts">
  import { onMount } from 'svelte';
  import Check from '@lucide/svelte/icons/check';
  import Plus from '@lucide/svelte/icons/plus';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import Modal from '@3pino/ui-kit-web/Modal.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import { keyName } from '../lib/format';
  import { ui } from '../lib/ui.svelte';
  import KeyIcon from './KeyIcon.svelte';

  // アプリ・サイトを選ぶポップアップ。候補（このデバイスのアプリ・その人が使ったことのあるもの）から選ぶか、サイトやアプリの名前を直接入れる
  let {
    title = 'アプリ・サイトを追加',
    candidates,
    picked,
    onpick,
    onunpick,
    onclose,
    appsOnly = false,
  }: {
    /** アプリだけを選ぶ（サイトは出さない。パッケージ名を直接入れられる） */
    appsOnly?: boolean;
    title?: string;
    candidates: { key: string; label: string }[];
    /** すでに選ばれているか */
    picked: (key: string) => boolean;
    onpick: (key: string, label: string) => void;
    onunpick: (key: string) => void;
    onclose: () => void;
  } = $props();

  let search = $state('');

  // 戻る操作（Android の戻る）で閉じる
  onMount(() => {
    // スマホでは、開いたときにキーボードを出さない（一覧が隠れるので、検索を押したときだけ出す）
    if (ui.isMobile) (document.activeElement as HTMLElement | null)?.blur();
    return ui.pushLayer(onclose);
  });

  const collator = new Intl.Collator('ja');
  const sorted = $derived([...candidates].sort((a, b) => collator.compare(a.label, b.label)));
  const q = $derived(search.trim().toLowerCase());
  const matched = $derived(q ? sorted.filter((c) => (c.label + ' ' + c.key).toLowerCase().includes(q)) : sorted);
  // 一度に描く行数を限る（数百件をいっぴんに作らない。残りは検索で絞り込む）
  const LIMIT = 80;
  const visible = $derived(matched.slice(0, LIMIT));

  /** 入力された文字を識別子にする。サイトの名前（youtube.com）か、`android:com.x` のような識別子だけ受け付ける */
  function toKey(input: string): string | null {
    const s = input.trim();
    if (!s) return null;
    if (appsOnly) {
      // パッケージ名（jp.ne.paypay.android.app）か、android:… だけ
      if (/^android:[A-Za-z0-9_.]+$/.test(s)) return s;
      return /^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$/.test(s) ? `android:${s}` : null;
    }
    if (/^(android|win|linux|domain):\S+$/.test(s)) return s;
    // URL が貼られたときは、ホスト名だけにする（パス・クエリは持たない）
    const host = s.replace(/^[a-z]+:\/\//i, '').split(/[/?#]/)[0].replace(/^www\./i, '').toLowerCase();
    return /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$/.test(host) ? `domain:${host}` : null;
  }

  const manualKey = $derived(toKey(search));
  const showManual = $derived(manualKey !== null && !candidates.some((c) => c.key === manualKey));

  function toggle(key: string, label: string) {
    if (picked(key)) onunpick(key);
    else onpick(key, label);
  }
</script>

<Modal {title} {onclose} width={460}>
  <TextInput
    bind:value={search}
    placeholder={appsOnly ? '検索、またはパッケージ名を入力' : '検索、またはサイト名（youtube.com）を入力'}
    autocapitalize="off"
  />

  <div class="picker-list scroll">
    {#if showManual && manualKey}
      {@const key = manualKey}
      <button type="button" class="pick-row" onclick={() => toggle(key, keyName(key))}>
        <KeyIcon {key} size={28} />
        <span class="pick-name">「{keyName(key)}」を追加</span>
        {#if picked(key)}<Check size={18} />{:else}<Plus size={18} />{/if}
      </button>
    {/if}
    {#each visible as c (c.key)}
      <button type="button" class="pick-row" class:pick-on={picked(c.key)} onclick={() => toggle(c.key, c.label)}>
        <KeyIcon key={c.key} size={28} />
        <span class="pick-name">
          {c.label}
          <span class="muted small">{c.key.startsWith('domain:') ? 'サイト' : keyName(c.key)}</span>
        </span>
        {#if picked(c.key)}<Check size={18} />{:else}<Plus size={18} />{/if}
      </button>
    {:else}
      {#if !showManual}
        <p class="empty">
          {#if appsOnly}
            {q ? '見つかりません。パッケージ名（jp.ne.paypay.android.app など）をそのまま入れると追加できます。' : '候補がありません。'}
          {:else}
            {q ? '見つかりません。サイト名（youtube.com など）をそのまま入れると追加できます。' : '候補がありません。サイト名（youtube.com など）を入れてください。'}
          {/if}
        </p>
      {/if}
    {/each}
    {#if matched.length > LIMIT}
      <p class="empty small">ほか {matched.length - LIMIT} 件。検索で絞り込んでください。</p>
    {/if}
  </div>

  <div class="row picker-footer">
    <Button variant="primary" onclick={onclose}>閉じる</Button>
  </div>
</Modal>

<style>
  .picker-list {
    display: flex;
    flex-direction: column;
    max-height: min(52vh, 420px);
    margin: var(--space-5) 0;
  }
  .pick-row {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    width: 100%;
    padding: var(--space-3) var(--space-3);
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: inherit;
    text-align: left;
    /* 画面の外の行は描かない（数百件あっても軽くする） */
    content-visibility: auto;
    contain-intrinsic-size: auto 46px;
  }
  .pick-row:hover {
    background: var(--hover);
  }
  .pick-on {
    color: var(--accent, inherit);
  }
  .pick-name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .picker-footer {
    justify-content: flex-end;
  }
</style>
