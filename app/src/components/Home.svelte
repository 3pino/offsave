<script lang="ts">
  import { onMount } from 'svelte';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import StatusLine from '@3pino/ui-kit-web/StatusLine.svelte';
  import { errorText } from '../lib/errors';
  import { dateTime } from '../lib/format';
  import { isAndroid, native } from '../lib/native';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { NativeStatus, UserStatus } from '../lib/types';
  import Page from './Page.svelte';
  import PersonStatus from './PersonStatus.svelte';

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let status = $state<UserStatus | null>(null);
  let ns = $state<NativeStatus | null>(null);

  async function refresh() {
    try {
      status = await api.status(saved.userId);
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
    if (isAndroid) {
      try {
        ns = await native.status();
      } catch (e) {
        ui.toast(errorText(e), 'error');
      }
    }
  }

  onMount(() => {
    void refresh();
    const t = setInterval(refresh, 3000);
    return () => clearInterval(t);
  });

  interface Check {
    ok: boolean;
    title: string;
    action: { label: string; run: () => Promise<void> } | null;
    hint: string;
  }

  // 次の 4 つを、上から順に有効にする
  const checks = $derived<Check[]>(
    ns
      ? [
          {
            ok: ns.accessibility,
            title: 'Accessibility（前面のアプリの検知と制限の表示）',
            action: ns.accessibility ? null : { label: '設定を開く', run: native.openAccessibilitySettings },
            hint: ns.accessibility
              ? ''
              : '一覧から offsave を選んで有効にします。「制限された設定」と出たら、「アプリ情報」の右上のメニューから許可してください。',
          },
          {
            ok: ns.deviceAdmin,
            title: '端末管理者（アンインストールの防止）',
            action: ns.deviceAdmin ? null : { label: '有効にする', run: native.requestDeviceAdmin },
            hint: '',
          },
          {
            ok: ns.batteryUnrestricted,
            title: '電池の最適化から除外（連絡が途切れないように）',
            action: ns.batteryUnrestricted ? null : { label: '許可する', run: native.requestBatteryExemption },
            hint: '',
          },
          {
            ok: ns.notifications,
            title: '通知（友達の通知を受け取る）',
            action: ns.notifications ? null : { label: '許可する', run: native.requestNotifications },
            hint: '',
          },
        ]
      : [],
  );
</script>

<Page title="{saved.userName} さん">
  {#if isAndroid && ns}
    <section class="home-block">
      <div class="row">
        <h2 class="block-title grow">このデバイス（{saved.deviceName}）</h2>
        {#if ns.connected}
          <span class="pill pill-ok">サーバーに接続中</span>
        {:else}
          <span class="pill pill-bad">未接続</span>
        {/if}
      </div>
      {#if !ns.connected && ns.lastError}
        <p class="empty">{ns.lastError}（制限は最後に受け取った内容で続きます）</p>
      {/if}
      {#if ns.unlockUntil > Date.now()}
        <StatusLine kind="warn">一時解除中（{dateTime(ns.unlockUntil)} まで）</StatusLine>
      {/if}
      {#each checks as c (c.title)}
        <SettingRow name={c.title} description={c.hint} icon={c.ok ? CircleCheck : CircleX}>
          {#snippet control()}
            <div class="setting-row-control-slot">
              {#if c.action}
                {@const a = c.action}
                <Button variant="primary" onclick={() => a.run()}>{a.label}</Button>
              {/if}
            </div>
          {/snippet}
        </SettingRow>
      {/each}
      {#if !ns.accessibility}
        <div><Button onclick={() => native.openAppDetails()}>アプリ情報（制限された設定の許可）</Button></div>
      {/if}
    </section>
  {/if}

  {#if status}
    <PersonStatus {status} />
  {/if}
</Page>

<style>
  .home-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
</style>
