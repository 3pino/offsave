import { SvelteMap } from 'svelte/reactivity';
import { names } from './names.svelte';
import { isAndroid, native } from './native';
import { session } from './session.svelte';

/** 取れなかったものを、取り直すまでの間隔 */
const RETRY_MS = 60_000;
/** 空いた時間が来なくても、これだけ待ったら 1 つ進める */
const IDLE_TIMEOUT_MS = 3000;
/** requestIdleCallback が無い環境で、次の 1 つまで空ける間隔 */
const FALLBACK_MS = 150;

type IdleWindow = Window & {
  requestIdleCallback?: (cb: () => void, opts?: { timeout: number }) => number;
};

/**
 * アイコン（識別子 → 画像の URL）。見た目の飾りなので、画面の操作（入力・スクロール）を邪魔しないことを優先する。
 * 画面に見えたものだけ、ブラウザが空いているときに 1 つずつ取る。
 * このデバイスのアプリは、デバイスから直接。それ以外（友達のアプリ・サイト）は、サーバーから。
 */
class Icons {
  // キーごとに反応する（1 つ増えても、そのアイコンだけが描き直される）
  private urls = new SvelteMap<string, string>();
  private failedAt = new Map<string, number>();
  private pending = new Set<string>();
  private queue: string[] = [];
  private running = false;
  private observer: IntersectionObserver | null = null;
  private watched = new Map<Element, string>();

  /** 取れていれば画像の URL。なければ null（取れるまでは、呼び出し側の代わりの絵を出す） */
  get(key: string): string | null {
    return this.urls.get(key) ?? null;
  }

  /** 要素が画面に見えたら、アイコンを取る（監視は全部で 1 つの IntersectionObserver にまとめる）。返す関数で監視をやめる */
  watch(el: Element, key: string): () => void {
    if (this.urls.has(key)) return () => {};
    if (typeof IntersectionObserver === 'undefined') {
      this.request(key);
      return () => {};
    }
    this.observer ??= new IntersectionObserver((entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        const k = this.watched.get(e.target);
        this.observer?.unobserve(e.target);
        this.watched.delete(e.target);
        if (k) this.request(k);
      }
    });
    this.watched.set(el, key);
    this.observer.observe(el);
    return () => {
      this.watched.delete(el);
      this.observer?.unobserve(el);
    };
  }

  private request(key: string) {
    if (this.urls.has(key) || this.pending.has(key)) return;
    const failed = this.failedAt.get(key);
    if (failed !== undefined && Date.now() - failed < RETRY_MS) return;
    this.pending.add(key);
    this.queue.push(key);
    this.schedule();
  }

  /** 空き時間に、1 つだけ取る。終わったらまた空き時間を待つ */
  private schedule() {
    if (this.running || this.queue.length === 0) return;
    this.running = true;
    const run = () => {
      // 画面から消えたものを取りに行かないよう、新しく見えたものから（後ろから）取る
      const key = this.queue.pop()!;
      void this.load(key).finally(() => {
        this.pending.delete(key);
        this.running = false;
        this.schedule();
      });
    };
    const w = window as IdleWindow;
    if (w.requestIdleCallback) w.requestIdleCallback(run, { timeout: IDLE_TIMEOUT_MS });
    else setTimeout(run, FALLBACK_MS);
  }

  private async load(key: string) {
    let blob: Blob | null = null;
    if (isAndroid && names.isLocalApp(key)) {
      try {
        const png = await native.appIcon(key.slice('android:'.length));
        if (png) blob = new Blob([Uint8Array.from(atob(png), (c) => c.charCodeAt(0))], { type: 'image/png' });
      } catch {
        /* サーバーのものを使う */
      }
    }
    if (!blob) blob = (await session.api?.icon(key)) ?? null;
    if (blob) this.urls.set(key, URL.createObjectURL(blob));
    else this.failedAt.set(key, Date.now());
  }
}

export const icons = new Icons();
