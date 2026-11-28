<script lang="ts">
  import { onMount } from 'svelte';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import Download from '@lucide/svelte/icons/download';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Eye from '@lucide/svelte/icons/eye';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import Smartphone from '@lucide/svelte/icons/smartphone';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import Users from '@lucide/svelte/icons/users';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import Section from '@3pino/ui-kit-web/Section.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import StatusLine from '@3pino/ui-kit-web/StatusLine.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import { errorText } from '../lib/errors';
  import { clock } from '../lib/format';
  import { parseTime, WEEK_LABELS } from '../lib/limits';
  import { isAndroid, native } from '../lib/native';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import { updater } from '../lib/updater.svelte';
  import type { UserSettings } from '../lib/types';
  import AndroidSettings from './AndroidSettings.svelte';
  import Friends from './Friends.svelte';
  import Page from './Page.svelte';
  import ProtectionPanel from './ProtectionPanel.svelte';
  import SettingsDevices from './SettingsDevices.svelte';
  import SettingsHidden from './SettingsHidden.svelte';

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let settings = $state<UserSettings | null>(null);
  let boundary = $state('04:00');
  let busy = $state(false);

  async function load() {
    try {
      const me = await api.me();
      settings = me.settings;
      boundary = clock(me.settings.day_boundary_min);
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  // 友達からの申請（承認待ち）があれば、「友達」の行で知らせる
  let incoming = $state(0);
  async function loadIncoming() {
    try {
      incoming = (await api.friends()).filter((f) => f.state === 'pending_in').length;
    } catch {
      /* 取れないときは、知らせない */
    }
  }

  onMount(() => {
    void load();
    void updater.loadVersion();
  });
  // 設定の一覧に戻ったときも取り直す
  $effect(() => {
    if (!ui.settingsSub) void loadIncoming();
  });

  const changed = $derived(settings !== null && parseTime(boundary) !== settings.day_boundary_min);

  async function saveSettings(next: UserSettings) {
    busy = true;
    try {
      // 友達に通知されるのは、タイムゾーンと 1 日の始まりの変更（週の始まりは表示だけの設定）
      const notified = settings !== null && (settings.timezone !== next.timezone || settings.day_boundary_min !== next.day_boundary_min);
      settings = await api.updateSettings(next);
      boundary = clock(settings.day_boundary_min);
      ui.toast(notified ? '保存しました（友達に通知されます）' : '保存しました');
    } catch (e) {
      ui.toast(errorText(e), 'error');
    } finally {
      busy = false;
    }
  }

  function saveBoundary() {
    const min = parseTime(boundary);
    if (!settings || min === null || min >= 1440) return ui.toast('1 日の始まりは 00:00〜23:59 で入れてください', 'error');
    void saveSettings({ ...settings, day_boundary_min: min });
  }

  // サブ画面を開いている間は、戻る操作（Android の戻る）で閉じる
  $effect(() => {
    if (!ui.settingsSub) return;
    return ui.pushLayer(() => (ui.settingsSub = null));
  });
  const closeSub = () => (ui.settingsSub = null);

  async function unlink() {
    const ok = await ui.confirm({
      title: '連携を解除しますか？',
      body: 'このデバイスは友達から見えなくなり、制限も外れます。友達には通知されます。',
      okLabel: '解除する',
      danger: true,
    });
    if (!ok) return;
    try {
      if (saved.deviceId) await api.removeDevice(saved.deviceId);
      if (isAndroid) await native.clear();
      session.clear();
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  const st = $derived(updater.state);
  const updating = $derived(st.kind === 'checking' || st.kind === 'downloading' || st.kind === 'installing');
  const percent = $derived(st.kind === 'downloading' && st.total ? Math.floor((st.downloaded / st.total) * 100) : null);
  const mb = (n: number) => (n / 1024 / 1024).toFixed(1);

  /** 手動で確認したときだけ、「最新です」をトーストで知らせる */
  let checkingByHand = false;
  function checkUpdate() {
    checkingByHand = true;
    void updater.check();
  }
  $effect(() => {
    if (st.kind === 'latest' && checkingByHand) {
      checkingByHand = false;
      ui.toast('最新です');
    }
  });
</script>

{#if ui.settingsSub === 'protection'}
  <Page title="このデバイスの監視" onback={closeSub}>
    <ProtectionPanel />
  </Page>
{:else if ui.settingsSub === 'android'}
  <AndroidSettings onback={closeSub} />
{:else if ui.settingsSub === 'devices'}
  <SettingsDevices onback={closeSub} onunlink={unlink} />
{:else if ui.settingsSub === 'hidden'}
  <SettingsHidden onback={closeSub} />
{:else if ui.settingsSub === 'friends'}
  <Friends onback={closeSub} />
{:else}
<Page title="設定" header={false}>
  <Section title="アカウント">
    <SettingRow name={saved.userName} />
    <SettingRow name="サーバー" description={saved.serverUrl} />
  </Section>

  <Section title="このアプリ">
    {#if isAndroid}
      <SettingRow name="このデバイスの監視" description="必要な権限" icon={ShieldCheck} onclick={() => (ui.settingsSub = 'protection')}>
        {#snippet control()}
          {#if protection.incomplete}<span class="pill pill-bad">要対応</span>{/if}
        {/snippet}
      </SettingRow>
      <SettingRow name="Android の設定" description="通知・データ・電池など" icon={SlidersHorizontal} onclick={() => (ui.settingsSub = 'android')} />
    {/if}
    <SettingRow name="友達" icon={Users} onclick={() => (ui.settingsSub = 'friends')}>
      {#snippet control()}
        {#if incoming > 0}<span class="pill pill-bad">承認待ち {incoming}</span>{/if}
      {/snippet}
    </SettingRow>
    <SettingRow name="デバイス" icon={Smartphone} onclick={() => (ui.settingsSub = 'devices')} />
    <SettingRow name="非表示のアプリ" icon={Eye} onclick={() => (ui.settingsSub = 'hidden')} />
  </Section>

  <Section title="日付と時間">
    {#if settings}
      <label class="field">
        <span class="field-label">1 日の始まり</span>
        <div class="row">
          <input class="input time" type="time" bind:value={boundary} />
          <Button variant="primary" disabled={busy || !changed} onclick={saveBoundary}>保存</Button>
        </div>
      </label>
      <label class="field">
        <span class="field-label">タイムゾーン</span>
        <div class="row">
          <div class="grow"><TextInput bind:value={settings.timezone} placeholder="Asia/Tokyo" /></div>
          <Button variant="primary" disabled={busy} onclick={() => saveSettings({ ...settings!, timezone: settings!.timezone.trim() })}>保存</Button>
        </div>
      </label>
      <label class="field">
        <span class="field-label">週の始まり</span>
        <select class="input week" value={settings.week_start} disabled={busy} onchange={(e) => saveSettings({ ...settings!, week_start: Number(e.currentTarget.value) })}>
          {#each WEEK_LABELS as w, i (i)}<option value={i}>{w}曜日</option>{/each}
        </select>
      </label>
      <p class="muted small">タイムゾーンと 1 日の始まりの変更は、友達に通知されます。週の始まりは、週の合計と棒グラフの表示だけに使います。</p>
    {/if}
  </Section>

  <!-- バージョンは一番下。新しい版や、うまくいかなかったときだけ、その状態と操作を出す（「最新です」はトーストで知らせる） -->
  <Section title="アプリについて">
    <SettingRow name="バージョン">
      {#snippet control()}
        <div class="version-control">
          <span class="version">{updater.supported ? (updater.version ? `v${updater.version}` : '…') : '開発版'}</span>
          {#if updater.supported}
            <IconButton label="アップデートを確認" title="アップデートを確認" disabled={updating} onclick={checkUpdate}>
              <RefreshCw size={18} class={st.kind === 'checking' ? 'spin' : ''} />
            </IconButton>
          {/if}
        </div>
      {/snippet}
    </SettingRow>

    {#if updater.supported && st.kind !== 'idle' && st.kind !== 'checking' && st.kind !== 'latest'}
      <div class="update-state">
        {#if st.kind === 'available'}
          <StatusLine kind="accent" icon={Download}>新しいバージョン v{st.version} があります</StatusLine>
          {#if st.notes}<p class="muted small notes">{st.notes}</p>{/if}
        {:else if st.kind === 'permission'}
          <StatusLine kind="warn" icon={CircleAlert}>「不明なアプリのインストール」を許可してから、戻って「続ける」を押してください</StatusLine>
        {:else if st.kind === 'downloading'}
          <StatusLine busy kind="muted">ダウンロード中… {#if st.total}{percent}%{:else}{mb(st.downloaded)} MB{/if}</StatusLine>
          <progress class="update-progress" max={st.total ?? undefined} value={st.total ? st.downloaded : undefined}></progress>
        {:else if st.kind === 'installing'}
          <StatusLine busy kind="muted">インストールしています…</StatusLine>
        {:else if st.kind === 'error'}
          <StatusLine kind="error" icon={CircleAlert}>{st.message}</StatusLine>
          <p class="muted small">うまくいかないときは、リリースページから直接ダウンロードできます。</p>
        {/if}

        {#if st.kind === 'available' || st.kind === 'permission' || st.kind === 'error'}
          <div class="row row-wrap">
            {#if st.kind === 'available'}
              <Button variant="primary" onclick={() => updater.install()}><Download size={15} />ダウンロードしてインストール</Button>
            {:else if st.kind === 'permission'}
              <Button variant="primary" onclick={() => updater.install()}>続ける</Button>
            {:else}
              <Button onclick={() => updater.openReleasePage()}><ExternalLink size={15} />リリースページを開く</Button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </Section>
</Page>
{/if}

<style>
  .update-state {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .update-state p {
    margin: 0;
  }
  .notes {
    white-space: pre-wrap;
  }
  .time {
    width: 130px;
  }
  .week {
    width: 130px;
  }
  .version-control {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .version {
    font-family: var(--mono);
    font-size: var(--fs-sm);
  }
  .update-progress {
    width: 100%;
    height: 8px;
    accent-color: var(--accent);
  }
</style>
