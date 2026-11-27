<script lang="ts">
  import { onMount } from 'svelte';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import Download from '@lucide/svelte/icons/download';
  import Eye from '@lucide/svelte/icons/eye';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import Smartphone from '@lucide/svelte/icons/smartphone';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import Section from '@3pino/ui-kit-web/Section.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import StatusLine from '@3pino/ui-kit-web/StatusLine.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import { errorText } from '../lib/errors';
  import { clock } from '../lib/format';
  import { parseTime } from '../lib/limits';
  import { isAndroid, native } from '../lib/native';
  import { protection } from '../lib/protection.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import { RELEASES_URL, updater } from '../lib/updater.svelte';
  import type { UserSettings } from '../lib/types';
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

  onMount(() => {
    void load();
    void updater.loadVersion();
  });

  const changed = $derived(settings !== null && parseTime(boundary) !== settings.day_boundary_min);

  async function saveSettings(next: UserSettings) {
    busy = true;
    try {
      settings = await api.updateSettings(next);
      boundary = clock(settings.day_boundary_min);
      ui.toast('保存しました（友達に通知されます）');
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
  const percent = $derived(st.kind === 'downloading' && st.total ? Math.floor((st.downloaded / st.total) * 100) : null);
</script>

{#if ui.settingsSub === 'protection'}
  <Page title="このデバイスの保護" onback={closeSub}>
    <ProtectionPanel />
  </Page>
{:else if ui.settingsSub === 'devices'}
  <SettingsDevices onback={closeSub} onunlink={unlink} />
{:else if ui.settingsSub === 'hidden'}
  <SettingsHidden onback={closeSub} />
{:else}
<Page title="設定">
  <Section title="アカウント">
    <SettingRow name={saved.userName} description="Tailscale のアカウントで、自動でログインしています。" />
    <SettingRow name="サーバー" description={saved.serverUrl} />
  </Section>

  <Section title="このアプリ">
    {#if isAndroid}
      <SettingRow name="このデバイスの保護" description="Accessibility・デバイス管理者・電池・通知、サーバーとの接続" icon={ShieldCheck} onclick={() => (ui.settingsSub = 'protection')}>
        {#snippet control()}
          {#if protection.incomplete}<span class="pill pill-bad">要対応</span>{/if}
        {/snippet}
      </SettingRow>
    {/if}
    <SettingRow name="デバイス" description="登録したデバイスの一覧と、削除" icon={Smartphone} onclick={() => (ui.settingsSub = 'devices')} />
    <SettingRow name="今日の利用の表示" description="ホーム画面のアプリなど、一覧に出さないもの" icon={Eye} onclick={() => (ui.settingsSub = 'hidden')} />
  </Section>

  <Section title="日付と時間">
    {#if settings}
      <label class="field">
        <span class="field-label">1 日の始まり（この時刻から次の同じ時刻までを 1 日として数えます）</span>
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
      <p class="muted small">変更は友達に通知されます。</p>
    {/if}
  </Section>

  {#if updater.supported}
    <Section title="アプリの更新">
      <SettingRow name="バージョン" description={updater.version ? `v${updater.version}` : ''} />
      <div class="update-state">
        {#if st.kind === 'checking'}
          <StatusLine busy kind="muted">確認しています…</StatusLine>
        {:else if st.kind === 'latest'}
          <StatusLine kind="ok" icon={CircleCheck}>最新です</StatusLine>
        {:else if st.kind === 'available'}
          <StatusLine kind="accent" icon={Download}>新しいバージョン v{st.version} があります</StatusLine>
          {#if st.notes}<p class="muted small notes">{st.notes}</p>{/if}
        {:else if st.kind === 'permission'}
          <StatusLine kind="warn" icon={CircleAlert}>「不明なアプリのインストール」を許可してから、もう一度「更新する」を押してください</StatusLine>
        {:else if st.kind === 'downloading'}
          <StatusLine busy kind="muted">ダウンロードしています{percent !== null ? `（${percent}%）` : ''}…</StatusLine>
        {:else if st.kind === 'installing'}
          <StatusLine busy kind="muted">インストールしています…</StatusLine>
        {:else if st.kind === 'error'}
          <StatusLine kind="error" icon={CircleAlert}>{st.message}</StatusLine>
        {/if}
        <div class="row row-wrap">
          {#if st.kind === 'available' || st.kind === 'permission'}
            <Button variant="primary" onclick={() => updater.install()}>更新する</Button>
          {/if}
          <Button disabled={st.kind === 'checking' || st.kind === 'downloading' || st.kind === 'installing'} onclick={() => updater.check()}>更新を確認</Button>
          {#if st.kind === 'error'}
            <Button onclick={() => updater.openReleasePage()}>リリースのページを開く</Button>
          {/if}
        </div>
        <p class="muted small">更新は <a href={RELEASES_URL}>GitHub のリリース</a>から、署名を確かめて入れます。</p>
      </div>
    </Section>
  {/if}

  <Section title="このデバイス">
    <SettingRow name={saved.deviceName || 'このデバイス'} description="連携を解除すると、このデバイスは友達から見えなくなり、制限も外れます。" />
    <div><Button variant="danger" onclick={unlink}>連携を解除する</Button></div>
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
</style>
