import { getItem, setItem } from '@3pino/app-kit-web/core/storage';

/**
 * 「今日の利用」に出さないもの（既定は、ホーム画面のアプリ・offsave 自身など、見ても意味のないもの）。
 * 表示だけの設定で、記録・制限には関係しない。このデバイスの画面だけの設定（友達の画面には影響しない）。
 */
export const DEFAULT_HIDDEN: string[] = [
  'android:dev.offsave.app',
  'android:com.android.systemui',
  'android:com.android.launcher3',
  'android:com.google.android.apps.nexuslauncher',
  'android:com.sec.android.app.launcher',
  'android:com.miui.home',
  'android:com.oppo.launcher',
  'android:com.huawei.android.launcher',
  'android:com.motorola.launcher3',
  'android:com.sonymobile.home',
  'android:net.oneplus.launcher',
  'android:com.nothing.launcher',
];

const KEY = 'offsave.hiddenUsage';

function load(): string[] {
  try {
    const v = getItem(KEY);
    if (v) {
      const parsed: unknown = JSON.parse(v);
      if (Array.isArray(parsed) && parsed.every((k) => typeof k === 'string')) return parsed;
    }
  } catch {
    /* 既定に戻す */
  }
  return [...DEFAULT_HIDDEN];
}

class Hidden {
  keys = $state.raw<string[]>(load());

  has = (key: string) => this.keys.includes(key);

  private set(keys: string[]) {
    this.keys = keys;
    setItem(KEY, JSON.stringify(keys));
  }

  add(key: string) {
    if (!this.has(key)) this.set([...this.keys, key]);
  }

  remove(key: string) {
    this.set(this.keys.filter((k) => k !== key));
  }

  reset() {
    this.set([...DEFAULT_HIDDEN]);
  }
}

export const hidden = new Hidden();
