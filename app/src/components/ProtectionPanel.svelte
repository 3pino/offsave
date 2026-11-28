<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import StatusLine from '@3pino/ui-kit-web/StatusLine.svelte';
  import { errorText } from '../lib/errors';
  import { dateTime } from '../lib/format';
  import { native } from '../lib/native';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';

  // 「設定の書き込みの権限」（WRITE_SECURE_SETTINGS）を付ける方法。Shizuku か、PC と USB でつないで adb
  type Os = 'windows' | 'mac' | 'linux';
  let os = $state<Os>('windows');
  let granting = $state(false);

  const grantCommand = $derived(
    `adb devices\nadb shell pm grant ${protection.ns?.packageName ?? 'dev.offsave.app'} android.permission.WRITE_SECURE_SETTINGS`,
  );
  /** adb（Android SDK Platform-Tools）を入れるコマンド（初回だけ） */
  const INSTALL: Record<Os, string> = {
    windows: 'winget install --id Google.PlatformTools',
    mac: 'brew install --cask android-platform-tools',
    linux: 'sudo apt install adb',
  };
  const OS_LABEL: Record<Os, string> = { windows: 'Windows', mac: 'macOS', linux: 'Linux' };

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      ui.toast('コピーしました');
    } catch {
      ui.toast('コピーできませんでした。画面の文字を選んでコピーしてください', 'error');
    }
  }

  const SHIZUKU_MESSAGE: Record<string, string> = {
    not_running: 'Shizuku が動いていません。Shizuku のアプリを開いて、開始してからもう一度試してください',
    too_old: 'Shizuku が古いです。更新してからもう一度試してください',
    denied: 'Shizuku で許可されませんでした。Shizuku のアプリで、offsave を許可してください',
    failed: '付けられませんでした。下の PC の手順で付けてください',
  };

  async function grantWithShizuku() {
    granting = true;
    try {
      const r = await native.grantWithShizuku();
      if (r === 'ok') ui.toast('付けました');
      else ui.toast(SHIZUKU_MESSAGE[r] ?? '付けられませんでした', 'error');
    } catch (e) {
      ui.toast(errorText(e), 'error');
    } finally {
      granting = false;
      await protection.refresh();
    }
  }

  // このデバイスの監視（Android）。上から順に有効にしていく
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
          {
            ok: ns.canWriteSecure,
            title: '設定の書き込みの権限（決済アプリなどのための一時停止）',
            action: null,
            hint: ns.canWriteSecure
              ? ''
              : ns.pauseApps > 0
                ? '友達が決めたアプリ（決済アプリなど）を使うには、この権限が必要です。下の方法で付けてください。'
                : 'PayPay など、Accessibility が有効だと使えないアプリを使うときだけ必要です（いまは無くても動きます）。',
          },
          {
            ok: ns.usageAccess,
            title: '使用状況へのアクセス（一時停止中に、前面のアプリを見る）',
            action: ns.usageAccess ? null : { label: '許可する', run: native.openUsageAccessSettings },
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

  {#if !ns.canWriteSecure}
    <h2 class="block-title">設定の書き込みの権限を付ける</h2>
    <p class="muted small">付けるのは 1 回だけです（offsave をアンインストールするまで有効）。更新しても消えません。</p>

    <h3 class="sub-title">方法 1: Shizuku（PC なし）</h3>
    <p class="muted small">
      {ns.shizuku ? 'Shizuku が動いています。' : 'Shizuku が動いていません（入れていない場合は、方法 2 で付けられます）。'}
      付けるときだけ使い、付けたあとは Shizuku を止めても構いません。
    </p>
    <div><Button variant="primary" disabled={granting} onclick={grantWithShizuku}>Shizuku で付ける</Button></div>

    <h3 class="sub-title">方法 2: PC と USB（adb）</h3>
    <ol class="steps muted small">
      <li>「設定 → デバイス情報」で「ビルド番号」を 7 回タップして、「開発者向けオプション」を出す。</li>
      <li>「開発者向けオプション」で「USB デバッグ」をオンにして、PC と USB でつなぐ。「USB デバッグを許可しますか？」と出たら許可する。</li>
      <li>PC に adb が無ければ、入れる（初回だけ。入れたあとは、ターミナルを開き直す）。</li>
      <li>次のコマンドを実行する。</li>
    </ol>
    <div class="row os-tabs">
      {#each Object.keys(OS_LABEL) as o (o)}
        <Button variant={os === o ? 'primary' : 'default'} onclick={() => (os = o as Os)}>{OS_LABEL[o as Os]}</Button>
      {/each}
    </div>
    <p class="muted small">adb を入れる</p>
    <code class="selectable adb-command">{INSTALL[os]}</code>
    <div><Button onclick={() => copy(INSTALL[os])}>コピー</Button></div>
    <p class="muted small">権限を付ける（スマホが見えているかを確かめてから、付けます）</p>
    <code class="selectable adb-command">{grantCommand}</code>
    <div><Button onclick={() => copy(grantCommand)}>コピー</Button></div>
  {/if}

  <div><Button onclick={() => native.openNotificationSettings()}>通知の種類ごとの設定</Button></div>
{:else if protection.error}
  <p class="error-text">{protection.error}</p>
{:else}
  <p class="empty">読み込み中…</p>
{/if}

<style>
  .sub-title {
    margin: var(--space-5) 0 var(--space-2);
    font-size: var(--fs-md);
  }
  .steps {
    margin: 0;
    padding-left: var(--space-6);
  }
  .os-tabs {
    gap: var(--space-3);
  }
  .adb-command {
    white-space: pre-wrap;
    display: block;
    padding: var(--space-4) var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-size: var(--fs-sm);
    word-break: break-all;
  }
</style>
