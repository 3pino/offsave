// アプリのアップデートの「仕組み」（Tauri のアップデーター・APK の取得・GitHub のリリース）を、土台の UpdateBackend へつなぐ。
// 土台（@3pino/app-kit-web/updater）は状態の機械だけ持つ。リポジトリ名はここで決める。
import { isAndroid } from '@3pino/app-kit-web/core/platform';
import { invokeNative } from '@3pino/app-kit-web/core/nativeInvoke';
import type { UpdateBackend } from '@3pino/app-kit-web/updater';

export const REPO = '3pino/offsave';
export const RELEASES_URL = `https://github.com/${REPO}/releases/latest`;

/** 「最新のリリース」のページの転送先（タグ）をデバイス側で読み、APK のアドレスを組み立てる（API の回数の上限にかからない） */
async function latestWithoutApi(): Promise<Awaited<ReturnType<UpdateBackend['fetchLatestAndroid']>>> {
  const { tag } = await invokeNative<{ tag: string }>('apk-updater', 'latest_tag');
  const version = tag.replace(/^v/, '');
  const name = `offsave_${version}_android-arm64.apk`;
  return {
    tag_name: tag,
    body: null,
    assets: [{ name, browser_download_url: `https://github.com/${REPO}/releases/download/${tag}/${name}` }],
  };
}

export function tauriUpdateBackend(): UpdateBackend {
  return {
    kind: () => (isAndroid() ? 'android' : 'desktop'),
    async version() {
      const { getVersion } = await import('@tauri-apps/api/app');
      return getVersion();
    },
    async checkDesktop() {
      const { check } = await import('@tauri-apps/plugin-updater');
      const update = await check({ headers: { 'Cache-Control': 'no-cache' } });
      return update ?? null;
    },
    async relaunch() {
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
    },
    async fetchLatestAndroid() {
      // GitHub の API（未認証は 1 時間に 60 回まで）。同じ IP の他の人が使うと 403 になるので、上限に当たったときは、API を使わない方法に切り替える
      let res: Response;
      try {
        res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
          headers: { Accept: 'application/vnd.github+json' },
          cache: 'no-store',
        });
      } catch {
        return latestWithoutApi();
      }
      if (res.status === 404) throw new Error('リリースが見つかりません');
      if (res.status === 403 || res.status === 429) return latestWithoutApi();
      if (!res.ok) throw new Error(`GitHub に接続できません (HTTP ${res.status})`);
      return (await res.json()) as Awaited<ReturnType<UpdateBackend['fetchLatestAndroid']>>;
    },
    async canInstallAndroid() {
      const { granted } = await invokeNative<{ granted: boolean }>('apk-updater', 'can_install');
      return granted;
    },
    async openInstallSettings() {
      await invokeNative('apk-updater', 'open_install_settings');
    },
    async downloadAndInstallAndroid(url, onProgress) {
      const { Channel } = await import('@tauri-apps/api/core');
      const channel = new Channel<{ downloaded: number; total: number | null }>();
      channel.onmessage = onProgress;
      await invokeNative('apk-updater', 'download_and_install', { url, onProgress: channel });
    },
    async openReleasePage() {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(RELEASES_URL);
    },
  };
}
