<script lang="ts">
  import type { Snippet } from 'svelte';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import PanelBar from '@3pino/ui-kit-web/PanelBar.svelte';

  // 1 画面の枠: 上の見出しの行（題名・戻る・右の操作）と、スクロールする本文
  let {
    title,
    onback,
    actions: pageActions,
    header = true,
    heading,
    children,
  }: {
    title: string;
    /** 渡すと、見出しの左に戻るボタンを出す */
    onback?: () => void;
    actions?: Snippet;
    /** false なら見出しの行を出さない（ナビで開く画面。ノッチを避ける隙間だけ空ける） */
    header?: boolean;
    /** 渡すと、題名（文字列）の代わりにこれを出す（題名を押して切り替える画面用。`title` は読み上げ用に残る） */
    heading?: Snippet;
    children: Snippet;
  } = $props();
</script>

<section class="page" aria-label={title}>
  {#if header}
    <PanelBar>
      {#if onback}<IconButton label="戻る" icon={ChevronLeft} onclick={onback} />{/if}
      {#if heading}{@render heading()}{:else}<h1 class="page-title">{title}</h1>{/if}
      {#snippet actions()}{@render pageActions?.()}{/snippet}
    </PanelBar>
  {/if}
  <div class="scroll page-body" class:page-headless={!header}>
    <div class="page-inner">{@render children()}</div>
  </div>
</section>
