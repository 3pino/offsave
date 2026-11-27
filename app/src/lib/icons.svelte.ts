import { names } from './names.svelte';
import { isAndroid, native } from './native';
import { session } from './session.svelte';

/** 取れなかったものを、取り直すまでの間隔 */
const RETRY_MS = 60_000;
/** 同時に取る数（一覧を開いたときに、いっぺんに頼まない） */
const CONCURRENCY = 4;

/**
 * アイコン（識別子 → 画像の URL）。画面に見えたものだけ `request` で取る。
 * このデバイスのアプリは、デバイスから直接。それ以外（友達のアプリ・サイト）は、サーバーから。
 */
class Icons {
  private urls = $state.raw<Record<string, string>>({});
  private failedAt = new Map<string, number>();
  private pending = new Set<string>();
  private queue: string[] = [];
  private running = 0;

  /** 取れていれば画像の URL。なければ null（取れるまでは、呼び出し側の代わりの絵を出す） */
  get(key: string): string | null {
    return this.urls[key] ?? null;
  }

  request(key: string) {
    if (this.urls[key] || this.pending.has(key)) return;
    const failed = this.failedAt.get(key);
    if (failed !== undefined && Date.now() - failed < RETRY_MS) return;
    this.pending.add(key);
    this.queue.push(key);
    this.pump();
  }

  private pump() {
    while (this.running < CONCURRENCY && this.queue.length > 0) {
      const key = this.queue.shift()!;
      this.running++;
      void this.load(key).finally(() => {
        this.running--;
        this.pending.delete(key);
        this.pump();
      });
    }
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
    if (blob) this.urls = { ...this.urls, [key]: URL.createObjectURL(blob) };
    else this.failedAt.set(key, Date.now());
  }
}

export const icons = new Icons();
