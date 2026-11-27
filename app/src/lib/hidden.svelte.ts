import { getItem, setItem } from '@3pino/app-kit-web/core/storage';

/**
 * 「今日の利用」で、初めは隠すアプリ・サイト（設定の「非表示のアプリ」）。既定は無し。
 * 表示だけの設定で、記録・制限には関係しない。このデバイスの画面だけの設定（友達の画面には影響しない）。
 * 隠したものも、「続きを表示」で見られる。
 */
const KEY = 'offsave.hiddenUsage';

function load(): string[] {
  try {
    const v = getItem(KEY);
    if (v) {
      const parsed: unknown = JSON.parse(v);
      if (Array.isArray(parsed) && parsed.every((k) => typeof k === 'string')) return parsed;
    }
  } catch {
    /* 無いものとして扱う */
  }
  return [];
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
}

export const hidden = new Hidden();
