<script lang="ts">
  import { Api } from './lib/api';
  import { isAndroid, native } from './lib/native';
  import { session } from './lib/session.svelte';

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
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<main style="padding: max(20px, env(safe-area-inset-top)) 20px 20px; padding-top: calc(env(safe-area-inset-top) + 40px); max-width: 520px; margin: 0 auto;">
  <h1>offsave</h1>
  <p class="muted">
    友達どうしで、お互いのスクリーンタイムを見守るアプリです。使うには、全員が合意していて、サーバーに Tailscale
    でつながっている必要があります。
  </p>

  <div class="card">
    <label for="server">サーバーのアドレス</label>
    <input id="server" placeholder="http://100.x.y.z:8090" bind:value={serverUrl} autocapitalize="off" />

    {#if isAndroid}
      <label for="dname">このデバイスの名前</label>
      <input id="dname" bind:value={deviceName} />
      <p class="muted">
        このデバイスは、友達から見えるようになります（使っているアプリ・時間・制限の解除・保護の無効化）。
      </p>
    {:else}
      <p class="muted">このデバイスでは、友達の状況を見るダッシュボードとして使います。</p>
    {/if}

    <details>
      <summary class="muted">開発用</summary>
      <label for="dev">ユーザー名（サーバーが開発モードのとき）</label>
      <input id="dev" placeholder="alice@example" bind:value={devUser} autocapitalize="off" />
    </details>

    {#if error}<div class="error">{error}</div>{/if}
    <div style="margin-top: 12px">
      <button class="primary" disabled={busy || !serverUrl.trim()} onclick={connect}>
        {busy ? '接続中…' : '同意して接続する'}
      </button>
    </div>
  </div>
</main>
