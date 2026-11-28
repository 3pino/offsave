import { getItem, setItem } from '@3pino/app-kit-web/core/storage';
import type { Api } from './api';
import { errorText } from './errors';
import { ui } from './ui.svelte';

/**
 * 「今日の利用」で、初めは隠すアプリ・サイト（設定の「非表示のアプリ」）。既定は無し。
 * 表示だけの設定で、記録・制限には関係しない。一覧はサーバーが持ち、本人と友達が同じものを見る。
 * 隠したものも、「非表示アプリを表示」「続きを表示」で見られる。
 *
 * `keys` は、いま画面で見ている人（`owner`）の一覧。状態を取るたびに `sync` で入れる。
 */
const LEGACY_KEY = 'offsave.hiddenUsage';

class Hidden {
  owner = $state<string | null>(null);
  keys = $state.raw<string[]>([]);
  /** 変更の送信中は、古い状態で上書きしない */
  private pending = 0;

  has = (key: string) => this.keys.includes(key);

  sync(owner: string, keys: string[] | undefined) {
    if (this.pending > 0) return;
    this.owner = owner;
    this.keys = keys ?? [];
  }

  /** すぐ画面に反映し、サーバーへ送る。失敗したら元に戻して知らせる */
  async change(api: Api, owner: string, add: string[], remove: string[]) {
    const before = this.keys;
    if (this.owner === owner) this.keys = [...before.filter((k) => !remove.includes(k)), ...add.filter((k) => !before.includes(k))];
    this.pending++;
    try {
      const keys = await api.changeHidden(owner, add, remove);
      if (this.owner === owner) this.keys = keys;
    } catch (e) {
      if (this.owner === owner) this.keys = before;
      ui.toast(errorText(e), 'error');
    } finally {
      this.pending--;
    }
  }

  /**
   * 以前は、このデバイスだけに持っていた。あれば、自分の一覧にサーバーで合わせる（和集合）。
   * 送れたら、このデバイスの分は消す。送れなければ、残して次に試す。
   */
  async migrateLegacy(api: Api, me: string) {
    let local: string[] = [];
    try {
      const parsed: unknown = JSON.parse(getItem(LEGACY_KEY) ?? '[]');
      if (Array.isArray(parsed)) local = parsed.filter((k): k is string => typeof k === 'string');
    } catch {
      /* 壊れていたら、無いものとして捨てる */
    }
    if (local.length > 0) {
      try {
        const keys = await api.changeHidden(me, local, []);
        if (this.owner === me && this.pending === 0) this.keys = keys;
      } catch {
        return;
      }
    }
    setItem(LEGACY_KEY, null);
  }
}

export const hidden = new Hidden();
