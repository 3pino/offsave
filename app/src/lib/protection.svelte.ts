import { errorText } from './errors';
import { isAndroid, native } from './native';
import type { NativeStatus } from './types';

/** 状態を取り直す間隔（設定画面から戻ってきたときは、すぐ取り直す） */
const POLL_MS = 5000;

/**
 * このデバイス（Android）の監視の状態: Accessibility・デバイス管理者・電池の最適化・通知・サーバーとの接続。
 * ホームの警告・ナビの点・設定の画面で共有する。Android 以外では、何もしない。
 */
class Protection {
  ns = $state.raw<NativeStatus | null>(null);
  error = $state('');
  private timer: ReturnType<typeof setInterval> | null = null;
  private onVisible = () => {
    if (document.visibilityState === 'visible') void this.refresh();
  };

  /** 足りないものがあるか（ホームの警告・設定の点に使う） */
  get incomplete(): boolean {
    const n = this.ns;
    if (!n) return false;
    // 友達が決めたアプリがあるのに、一時停止の準備（権限）が無いと、そのアプリを使う前に Accessibility を手で切ることになる
    const pauseReady = n.pauseApps === 0 || (n.canWriteSecure && n.usageAccess);
    return !((n.accessibility || n.paused) && n.deviceAdmin && n.batteryUnrestricted && n.notifications && pauseReady);
  }

  async refresh() {
    if (!isAndroid) return;
    try {
      this.ns = await native.status();
      this.error = '';
    } catch (e) {
      this.error = errorText(e);
    }
  }

  start() {
    if (!isAndroid || this.timer) return;
    void this.refresh();
    this.timer = setInterval(() => void this.refresh(), POLL_MS);
    document.addEventListener('visibilitychange', this.onVisible);
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    document.removeEventListener('visibilitychange', this.onVisible);
    this.ns = null;
  }
}

export const protection = new Protection();
