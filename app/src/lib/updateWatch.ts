import { getItem, setItem } from '@3pino/app-kit-web/core/storage';
import {
  canBackgroundCheck,
  isCheckDue,
  UPDATE_CHECK_FOCUS_GAP_MS,
  UPDATE_CHECK_INTERVAL_MS,
  versionToNotify,
} from '@3pino/app-kit-web/updater';
import { ui } from './ui.svelte';
import { updater } from './updater.svelte';

// アプリのアップデートを裏で確認し、新しい版があればトーストで知らせる（Tauri 版だけ）。
// 起動時・6 時間おき・フォーカスが戻ったとき（前回から 1 時間たっていれば）に確認する。同じ版は 1 回だけ知らせる

const CHECKED_AT_KEY = 'offsave.updateCheckedAt';
const NOTIFIED_KEY = 'offsave.updateNotifiedVersion';
/** 定期の確認の時計の刻み（間隔の判定は isCheckDue で行う） */
const TICK_MS = 30 * 60 * 1000;

function lastCheckedAt(): number | null {
  const raw = getItem(CHECKED_AT_KEY);
  const n = raw === null ? NaN : Number(raw);
  return Number.isFinite(n) ? n : null;
}

function notifyNewVersion(version: string): void {
  setItem(NOTIFIED_KEY, version);
  ui.toast(`新しいバージョン v${version} があります`, 'info', { label: '設定を開く', run: () => ui.selectTab('settings') }, 15_000);
}

/** 確認を始める。戻り値の関数で止める */
export function startUpdateWatch(): () => void {
  if (!updater.supported) return () => {};

  async function run(gapMs: number): Promise<void> {
    if (!isCheckDue(Date.now(), lastCheckedAt(), gapMs)) return;
    if (!canBackgroundCheck(updater.state)) return;
    setItem(CHECKED_AT_KEY, String(Date.now()));
    await updater.check({ quiet: true });
    const version = versionToNotify(updater.state, getItem(NOTIFIED_KEY));
    if (version) notifyNewVersion(version);
  }

  // 起動時は必ず確認する
  void run(0);
  const timer = setInterval(() => void run(UPDATE_CHECK_INTERVAL_MS), TICK_MS);

  const onVisible = () => {
    if (document.visibilityState === 'visible') void run(UPDATE_CHECK_FOCUS_GAP_MS);
  };
  document.addEventListener('visibilitychange', onVisible);
  window.addEventListener('focus', onVisible);

  return () => {
    clearInterval(timer);
    document.removeEventListener('visibilitychange', onVisible);
    window.removeEventListener('focus', onVisible);
  };
}
