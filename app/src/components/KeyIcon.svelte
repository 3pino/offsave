<script lang="ts">
  import { onMount } from 'svelte';
  import Globe from '@lucide/svelte/icons/globe';
  import { icons } from '../lib/icons.svelte';
  import { names } from '../lib/names.svelte';

  // アプリのアイコン・サイトのファビコン。画面に見えたときに取る。取れるまで（取れなければ）、代わりの絵を出す
  let { key, size = 28 }: { key: string; size?: number } = $props();

  let el: HTMLElement | undefined = $state();
  const url = $derived(icons.get(key));
  const isSite = $derived(key.startsWith('domain:'));
  const initial = $derived([...names.label(key)][0]?.toUpperCase() ?? '?');

  onMount(() => (el ? icons.watch(el, key) : undefined));
</script>

<span class="key-icon" bind:this={el} style:width="{size}px" style:height="{size}px" aria-hidden="true">
  {#if url}
    <img src={url} alt="" width={size} height={size} loading="lazy" decoding="async" />
  {:else if isSite}
    <Globe size={Math.round(size * 0.65)} />
  {:else}
    <span class="initial" style:font-size="{Math.round(size * 0.5)}px">{initial}</span>
  {/if}
</span>

<style>
  .key-icon {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 22%;
    overflow: hidden;
    background: var(--surface-2, var(--bg));
    color: var(--text-muted);
  }
  .key-icon img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .initial {
    font-weight: var(--fw-semibold);
  }
</style>
