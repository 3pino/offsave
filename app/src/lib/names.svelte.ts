import { isAndroid, native } from './native';
import { keyName } from './format';
import type { InstalledApp, KeyLabel } from './types';

/**
 * アプリ名（`android:<package>` → 表示名）。
 * このデバイスのアプリは、インストール済みの一覧から。ほかのデバイス（友達のものを含む）のアプリは、
 * そのデバイスがサーバーに送った表示名から。どちらも無ければパッケージ名のまま出す。
 */
class Names {
  apps = $state<InstalledApp[]>([]);
  private remote = $state<Record<string, string>>({});
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
    for (const l of labels) {
      if (this.remote[l.key] !== l.label) this.remote[l.key] = l.label;
    }
  }

  label(key: string): string {
    return this.local.get(key) ?? this.remote[key] ?? keyName(key);
  }
}

export const names = new Names();
