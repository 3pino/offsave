<script lang="ts">
  import { onMount } from 'svelte';
  import { isAndroid, native } from './lib/native';
  import { session } from './lib/session.svelte';
  import { dateTime } from './lib/format';
  import RuleEditor from './RuleEditor.svelte';
  import StatusView from './StatusView.svelte';
  import type { NativeStatus, RuleInput, UserSettings, UserStatus } from './lib/types';

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let status = $state<UserStatus | null>(null);
  let ns = $state<NativeStatus | null>(null);
  let error = $state('');
  let adding = $state(false);
  let settings = $state<UserSettings | null>(null);
  let settingsOpen = $state(false);

  async function refresh() {
    try {
      status = await api.status(saved.userId);
      settings ??= { ...status.settings };
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    if (isAndroid) {
      try {
        ns = await native.status();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
    }
  }

  onMount(() => {
    refresh();
    const t = setInterval(refresh, 3000);
    return () => clearInterval(t);
  });

  async function addRule(r: RuleInput) {
    await api.createRule(saved.userId, r);
    adding = false;
    await refresh();
  }

  async function saveSettings() {
    if (!settings) return;
    try {
      await api.updateSettings(settings);
      settingsOpen = false;
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function unlink() {
    if (!confirm('連携を解除しますか？\nこのデバイスは友達から見えなくなり、制限も外れます。友達には通知されます。')) return;
    try {
      if (saved.deviceId) await api.removeDevice(saved.deviceId);
      if (isAndroid) await native.clear();
      session.clear();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  interface Check {
    ok: boolean;
    title: string;
    action: { label: string; run: () => Promise<void> } | null;
    hint: string;
  }

  const checks = $derived<Check[]>(
    ns
      ? [
          // 次の 4 つを左から順に表示する
          {
            ok: ns.accessibility,
            title: 'Accessibility（前面のアプリの検知と制限の表示）',
            action: ns.accessibility ? null : { label: '設定を開く', run: native.openAccessibilitySettings },
            hint: ns.accessibility
              ? ''
              : '一覧から offsave を選んで有効にします。「制限された設定」と出たら、下の「アプリ情報」の右上のメニューから許可してください。',
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

<h1>{saved.userName} さん</h1>
{#if error}<div class="error">{error}</div>{/if}

{#if isAndroid && ns}
  <div class="card">
    <div class="row">
      <b class="grow">このデバイス（{saved.deviceName}）</b>
      {#if ns.connected}
        <span class="badge ok">サーバーに接続中</span>
      {:else}
        <span class="badge bad">未接続</span>
      {/if}
    </div>
    {#if !ns.connected && ns.lastError}<div class="muted">{ns.lastError}（制限は最後に受け取った内容で続きます）</div>{/if}
    {#if ns.unlockUntil > Date.now()}<div class="warn">一時解除中（{dateTime(ns.unlockUntil)} まで）</div>{/if}

    {#each checks as c}
      <div style="margin-top: 8px">
        <div class="row">
          <span class={c.ok ? 'ok' : 'bad'}>{c.ok ? '✓' : '✗'}</span>
          <div class="grow">{c.title}</div>
          {#if c.action}{@const a = c.action}<button class="small primary" onclick={() => a.run()}>{a.label}</button>{/if}
        </div>
        {#if c.hint}<div class="muted">{c.hint}</div>{/if}
      </div>
    {/each}
    {#if !ns.accessibility}
      <div style="margin-top: 8px"><button class="small" onclick={() => native.openAppDetails()}>アプリ情報（制限された設定の許可）</button></div>
    {/if}
  </div>
{/if}

{#if status}
  <StatusView {status} mine={true} />

  <h2>自分のルールを足す</h2>
  <p class="muted">自分で足せるのは、制限を厳しくするルールだけです。あとから外すには友達の操作が必要です。</p>
  {#if adding}
    <RuleEditor usage={status.usage_today} onSave={addRule} onCancel={() => (adding = false)} />
  {:else}
    <button onclick={() => (adding = true)}>ルールを追加</button>
  {/if}

  <h2>設定</h2>
  <div class="card">
    {#if settingsOpen && settings}
      <label for="tz">タイムゾーン</label>
      <input id="tz" bind:value={settings.timezone} />
      <label for="db">1日の始まり（分。240 = 4:00）</label>
      <input id="db" type="number" min="0" max="1439" bind:value={settings.day_boundary_min} />
      <p class="muted">変更は友達に通知されます。</p>
      <div class="row">
        <button class="primary" onclick={saveSettings}>保存</button>
        <button onclick={() => (settingsOpen = false)}>キャンセル</button>
      </div>
    {:else}
      <div class="row">
        <div class="grow">
          {status.settings.timezone} ・ 1日の始まり {String(Math.floor(status.settings.day_boundary_min / 60)).padStart(2, '0')}:{String(status.settings.day_boundary_min % 60).padStart(2, '0')}
        </div>
        <button class="small" onclick={() => (settingsOpen = true)}>変更</button>
      </div>
    {/if}
  </div>

  <div style="margin-top: 24px">
    <button class="danger" onclick={unlink}>連携を解除する</button>
  </div>
{/if}
