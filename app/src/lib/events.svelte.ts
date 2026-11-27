import { getItem, setItem } from '@3pino/app-kit-web/core/storage';
import { session } from './session.svelte';
import type { OffsaveEvent } from './types';

const SEEN_KEY = 'offsave.eventsSeenAt';

/** 通知（友達と自分のイベント）。ナビゲーションの点（新しいものがある）のために、画面を開いていなくても取り直す。 */
class Events {
  items = $state<OffsaveEvent[]>([]);
  error = $state('');
  /** 通知の画面を最後に見た時刻 */
  private seenAt = $state(Number(getItem(SEEN_KEY) ?? 0));
  private timer: ReturnType<typeof setInterval> | null = null;

  /** 自分以外の、まだ見ていない通知の数 */
  get unseen(): number {
    const me = session.saved?.userId;
    return this.items.filter((e) => e.at > this.seenAt && e.user.id !== me).length;
  }

  async refresh() {
    const api = session.api;
    if (!api) return;
    try {
      this.items = await api.events(100);
      this.error = '';
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  markSeen() {
    const latest = this.items[0]?.at ?? Date.now();
    this.seenAt = Math.max(this.seenAt, latest);
    setItem(SEEN_KEY, String(this.seenAt));
  }

  start() {
    if (this.timer) return;
    void this.refresh();
    this.timer = setInterval(() => void this.refresh(), 10_000);
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    this.items = [];
  }
}

export const events = new Events();
