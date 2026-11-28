import { untrack } from 'svelte';
import { isAndroid, native } from './native';
import { keyName } from './format';
import type { InstalledApp, KeyLabel, UserStatus } from './types';

/**
 * アプリ名（`android:<package>` → 表示名）。
 * このデバイスのアプリは、インストール済みの一覧から。ほかのデバイス（友達のものを含む）のアプリは、
 * そのデバイスがサーバーに送った表示名から。どちらも無ければパッケージ名のまま出す。
 */
class Names {
  /** インストール済みのアプリ（数百件あるので、中身まで反応させない） */
  apps = $state.raw<InstalledApp[]>([]);
  private remote = $state.raw<Record<string, string>>({});
  private local = $derived(new Map(this.apps.map((a) => [`android:${a.package}`, a.label])));

  async load() {
    if (!isAndroid) return;
    try {
      this.apps = await native.installedApps();
    } catch {
      /* 取れなくても動く */
    }
  }

  /** サーバーから受け取った表示名を覚える。 */
  learn(labels: KeyLabel[]) {
    untrack(() => {
      let next: Record<string, string> | null = null;
      for (const l of labels) {
        if (this.remote[l.key] !== l.label) (next ??= { ...this.remote })[l.key] = l.label;
      }
      if (next) this.remote = next;
    });
  }

  /** このデバイスにインストールされているアプリか */
  isLocalApp(key: string): boolean {
    return this.local.has(key);
  }

  label(key: string): string {
    return this.local.get(key) ?? this.remote[key] ?? keyName(key);
  }

  /** その人のルールの対象に選べるもの: 自分ならこのデバイスのアプリ、それに加えて、その人が使ったことのあるもの */
  candidates(status: UserStatus | null, mine: boolean): { key: string; label: string }[] {
    const map = new Map<string, string>();
    if (mine) for (const a of this.apps) map.set(`android:${a.package}`, a.label);
    for (const l of status?.labels ?? []) if (!map.has(l.key)) map.set(l.key, l.label);
    for (const u of status?.usage_today ?? []) if (!map.has(u.key)) map.set(u.key, this.label(u.key));
    return [...map].map(([key, label]) => ({ key, label }));
  }
}

export const names = new Names();
