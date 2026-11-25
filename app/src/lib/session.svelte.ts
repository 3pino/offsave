import { Api } from './api';

/** このデバイスの接続情報（登録済みなら入っている）。localStorage に保存する。 */
export interface Saved {
  serverUrl: string;
  devUser: string | null;
  deviceId: string;
  token: string;
  userId: string;
  userName: string;
  deviceName: string;
}

const KEY = 'offsave.session';

function load(): Saved | null {
  try {
    const v = localStorage.getItem(KEY);
    return v ? (JSON.parse(v) as Saved) : null;
  } catch {
    return null;
  }
}

class Session {
  saved = $state<Saved | null>(load());

  get api(): Api | null {
    return this.saved ? new Api(this.saved.serverUrl, this.saved.devUser) : null;
  }

  save(s: Saved) {
    localStorage.setItem(KEY, JSON.stringify(s));
    this.saved = s;
  }

  clear() {
    localStorage.removeItem(KEY);
    this.saved = null;
  }
}

export const session = new Session();
