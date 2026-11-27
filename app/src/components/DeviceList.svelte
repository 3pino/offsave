<script lang="ts">
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import { ago } from '../lib/format';
  import { names } from '../lib/names.svelte';
  import type { DeviceInfo } from '../lib/types';

  // デバイスの一覧（状態・最後の連絡・保護の不足）。onremove を渡すと、削除ボタンを出す
  let {
    devices,
    currentId = '',
    onremove,
  }: {
    devices: DeviceInfo[];
    /** いま使っているデバイスの ID（印を付ける） */
    currentId?: string;
    onremove?: (d: DeviceInfo) => void;
  } = $props();

  const STATE: Record<string, [string, string]> = {
    online: ['接続中', 'pill-ok'],
    suspect: ['連絡なし', 'pill-bad'],
    offline: ['停止', 'pill-warn'],
    new: ['未接続', 'pill-muted'],
  };
</script>

{#each devices as d (d.id)}
  {@const s = STATE[d.state] ?? [d.state, 'pill-muted']}
  <div class="card">
    <div class="card-title">
      <span class="grow">{d.name}{d.id === currentId ? '（このデバイス）' : ''}</span>
      <span class="pill {s[1]}">{s[0]}</span>
      {#if onremove}<IconButton label="{d.name} を削除" icon={Trash2} onclick={() => onremove(d)} />{/if}
    </div>
    <div class="muted small">
      {d.platform} ・ 最後の連絡 {d.last_seen ? ago(d.last_seen) : 'なし'}
      {#if d.foreground}・ いま {names.label(d.foreground)}{/if}
    </div>
    {#if d.flags.paused}
      <div class="muted small">決済アプリなどのため、保護を一時停止しています。</div>
    {:else if d.state !== 'new' && !(d.flags.accessibility && d.flags.device_admin)}
      <div class="error-text small">
        保護が不完全です:
        {#if !d.flags.accessibility}Accessibility が無効 {/if}
        {#if !d.flags.device_admin}デバイス管理者が無効{/if}
      </div>
    {/if}
  </div>
{:else}
  <p class="empty">デバイスがありません。</p>
{/each}
