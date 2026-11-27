import type {
  DeviceInfo,
  Friend,
  OffsaveEvent,
  Override,
  Rule,
  RuleInput,
  User,
  UserSettings,
  UserStatus,
} from './types';

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

/** サーバーの REST（/api）。認証は Tailscale が行うので、ヘッダーは付けない（開発モードのときだけ X-Dev-User）。 */
export class Api {
  constructor(
    public base: string,
    public devUser: string | null = null,
  ) {
    this.base = base.trim().replace(/\/+$/, '');
  }

  private async req<T>(method: string, path: string, body?: unknown): Promise<T> {
    const headers: Record<string, string> = {};
    if (body !== undefined) headers['content-type'] = 'application/json';
    if (this.devUser) headers['x-dev-user'] = this.devUser;
    let res: Response;
    try {
      res = await fetch(`${this.base}/api${path}`, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch (e) {
      throw new ApiError(0, `サーバーに接続できません（${this.base}）: ${e instanceof Error ? e.message : e}`);
    }
    const text = await res.text();
    let json: unknown = null;
    try {
      json = text ? JSON.parse(text) : null;
    } catch {
      /* 本文が JSON でない */
    }
    if (!res.ok) {
      const message = (json as { message?: string } | null)?.message ?? `HTTP ${res.status}`;
      throw new ApiError(res.status, message);
    }
    return json as T;
  }

  me = () => this.req<{ user: User; settings: UserSettings }>('GET', '/me');
  updateSettings = (s: UserSettings) => this.req<UserSettings>('PUT', '/me/settings', s);

  friends = () => this.req<Friend[]>('GET', '/friends');
  /** 友達にできる人（offsave を使っていて、まだ友達でも申請中でもない人） */
  people = () => this.req<User[]>('GET', '/people');
  requestFriend = (login_name: string) => this.req<Friend>('POST', '/friends', { login_name });
  acceptFriend = (id: string) => this.req<Friend>('POST', `/friends/${id}/accept`, {});
  removeFriend = (id: string) => this.req<{ ok: boolean }>('DELETE', `/friends/${id}`);

  registerDevice = (name: string, platform: string) =>
    this.req<{ device: DeviceInfo; token: string }>('POST', '/devices', { name, platform });
  removeDevice = (id: string) => this.req<{ ok: boolean }>('DELETE', `/devices/${id}`);

  status = (userId: string) => this.req<UserStatus>('GET', `/users/${userId}/status`);
  createRule = (userId: string, rule: RuleInput) => this.req<Rule>('POST', `/users/${userId}/rules`, rule);
  updateRule = (id: string, rule: RuleInput) => this.req<Rule>('PUT', `/rules/${id}`, rule);
  deleteRule = (id: string) => this.req<{ ok: boolean }>('DELETE', `/rules/${id}`);
  createOverride = (userId: string, date: string, rule_id: string | null) =>
    this.req<Override>('POST', `/users/${userId}/overrides`, { date, rule_id });
  deleteOverride = (id: string) => this.req<{ ok: boolean }>('DELETE', `/overrides/${id}`);

  events = (limit = 50) => this.req<OffsaveEvent[]>('GET', `/events?limit=${limit}`);

  /** アイコン（アプリは、デバイスが送ったもの。サイトは、サーバーが取ったファビコン）。無ければ null */
  async icon(key: string): Promise<Blob | null> {
    const headers: Record<string, string> = {};
    if (this.devUser) headers['x-dev-user'] = this.devUser;
    try {
      const res = await fetch(`${this.base}/api/icons?key=${encodeURIComponent(key)}`, { headers });
      return res.ok ? await res.blob() : null;
    } catch {
      return null;
    }
  }
}
