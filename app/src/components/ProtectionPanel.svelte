<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import StatusLine from '@3pino/ui-kit-web/StatusLine.svelte';
  import { dateTime } from '../lib/format';
  import { native } from '../lib/native';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';

  /** 設定の書き込みの権限を付ける adb のコマンド */
  const adbCommand = $derived(`adb shell pm grant ${protection.ns?.packageName ?? 'dev.offsave.app'} android.permission.WRITE_SECURE_SETTINGS`);

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(adbCommand);
      ui.toast('コピーしました');
    } catch {
      ui.toast('コピーできませんでした。画面の文字を選んでコピーしてください', 'error');
    }
  }

  // このデバイスの保護（Android）。上から順に有効にしていく
  const ns = $derived(protection.ns);
  const saved = $derived(session.saved!);

  interface Check {
    ok: boolean;
    title: string;
    action: { label: string; run: () => Promise<void> } | null;
    hint: string;
  }

  const checks = $derived<Check[]>(
    ns
      ? [
          {
            ok: ns.accessibility || ns.paused,
            title: 'Accessibility（前面のアプリの検知と制限の表示）',
            action: ns.accessibility || ns.paused ? null : { label: '設定を開く', run: native.openAccessibilitySettings },
            hint: ns.paused
              ? '決済アプリなどのため、いま一時停止中です（アプリを離れると戻ります）。'
              : ns.accessibility
                ? ''
              : '一覧から offsave を選んで有効にします。「制限された設定」と出たら、「アプリ情報」の右上のメニューから許可してください。',
          },
          {
            ok: ns.deviceAdmin,
            title: 'デバイス管理者（アンインストールの防止）',
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

{#if ns}
  <div class="row">
    <div class="grow muted">このデバイス（{saved.deviceName}）</div>
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
  {#if !ns.accessibility && !ns.paused}
    <div><Button onclick={() => native.openAppDetails()}>アプリ情報（制限された設定の許可）</Button></div>
  {/if}

  <!-- 友達が「保護を一時停止するアプリ」を決めているときだけ、その準備を出す -->
  {#if ns.pauseApps > 0}
    <h2 class="block-title">保護の一時停止の準備</h2>
    <SettingRow
      name="設定の書き込みの権限"
      description="PC と USB でつないで、次のコマンドを 1 回実行します（アンインストールするまで有効）。"
      icon={ns.canWriteSecure ? CircleCheck : CircleX}
    />
    {#if !ns.canWriteSecure}
      <code class="selectable adb-command">{adbCommand}</code>
      <div><Button onclick={copyCommand}>コマンドをコピー</Button></div>
    {/if}
    <SettingRow name="使用状況へのアクセス" icon={ns.usageAccess ? CircleCheck : CircleX}>
      {#snippet control()}
        <div class="setting-row-control-slot">
          {#if !ns.usageAccess}<Button variant="primary" onclick={() => native.openUsageAccessSettings()}>許可する</Button>{/if}
        </div>
      {/snippet}
    </SettingRow>
  {/if}
{:else if protection.error}
  <p class="error-text">{protection.error}</p>
{:else}
  <p class="empty">読み込み中…</p>
{/if}

<style>
  .adb-command {
    display: block;
    padding: var(--space-4) var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-size: var(--fs-sm);
    word-break: break-all;
  }
</style>
