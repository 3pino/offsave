<script lang="ts">
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import { Api } from '../lib/api';
  import { errorText } from '../lib/errors';
  import { isAndroid, native } from '../lib/native';
  import { session } from '../lib/session.svelte';

  let serverUrl = $state('');
  let devUser = $state('');
  let deviceName = $state(isAndroid ? 'Android' : 'このパソコン');
  let busy = $state(false);
  let error = $state('');

  async function connect() {
    busy = true;
    error = '';
    try {
      const api = new Api(serverUrl, devUser.trim() || null);
      const me = await api.me();
      let deviceId = '';
      let token = '';
      // 制限を強制できるのは Android だけ（デスクトップ版は、友達の状況を見るダッシュボードとして使う）
      if (isAndroid) {
        const r = await api.registerDevice(deviceName.trim() || 'Android', 'android');
        deviceId = r.device.id;
        token = r.token;
      }
      const saved = {
        serverUrl: api.base,
        devUser: api.devUser,
        deviceId,
        token,
        userId: me.user.id,
        userName: me.user.display_name,
        deviceName: deviceName.trim(),
      };
      if (isAndroid) {
        await native.configure({
          serverUrl: saved.serverUrl,
          devUser: saved.devUser,
          deviceId,
          token,
          userId: saved.userId,
          guardSettings: true,
        });
      }
      session.save(saved);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="setup scroll">
  <main class="setup-inner">
    <h1 class="setup-title">offsave</h1>
    <p class="muted">
      友達どうしで、お互いのスクリーンタイムを見守るアプリです。使うには、全員が合意していて、サーバーに Tailscale
      でつながっている必要があります。
    </p>

    <div class="card">
      <label class="field">
        <span class="field-label">サーバーのアドレス</span>
        <TextInput bind:value={serverUrl} placeholder="http://100.x.y.z:8090" autocapitalize="off" />
      </label>

      {#if isAndroid}
        <label class="field">
          <span class="field-label">このデバイスの名前</span>
          <TextInput bind:value={deviceName} />
        </label>
        <p class="muted small">
          このデバイスは、友達から見えるようになります（使っているアプリ・時間・制限の解除・保護の無効化）。
        </p>
      {:else}
        <p class="muted small">このデバイスでは、友達の状況を見るダッシュボードとして使います。</p>
      {/if}

      <details>
        <summary class="muted small">開発用</summary>
        <label class="field">
          <span class="field-label">ユーザー名（サーバーが開発モードのとき）</span>
          <TextInput bind:value={devUser} placeholder="alice@example" autocapitalize="off" />
        </label>
      </details>

      {#if error}<p class="error-text">{error}</p>{/if}
      <div>
        <Button variant="primary" disabled={busy || !serverUrl.trim()} onclick={connect}>
          {busy ? '接続中…' : '同意して接続する'}
        </Button>
      </div>
    </div>
  </main>
</div>

<style>
  .setup {
    height: 100%;
  }
  .setup-inner {
    max-width: 520px;
    margin: 0 auto;
    padding: calc(env(safe-area-inset-top) + var(--space-8)) var(--space-7) var(--space-8);
    display: flex;
    flex-direction: column;
    gap: var(--space-7);
  }
  .setup-title {
    margin: 0;
    font-size: var(--fs-2xl);
    font-weight: var(--fw-bold);
  }
  .setup-inner p {
    margin: 0;
  }
</style>
