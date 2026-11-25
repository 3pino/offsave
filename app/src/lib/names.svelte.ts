import { isAndroid, native } from './native';
import { keyName } from './format';
import type { InstalledApp } from './types';

/** このデバイスで分かるアプリ名（`android:<package>` → 表示名）。友達のデバイスのアプリは分からないのでパッケージ名のまま出す。 */
class Names {
  apps = $state<InstalledApp[]>([]);
  private map = $derived(new Map(this.apps.map((a) => [`android:${a.package}`, a.label])));

  async load() {
    if (!isAndroid) return;
    try {
      this.apps = await native.installedApps();
    } catch {
      /* 取れなくても動く */
    }
  }

  label(key: string): string {
    return this.map.get(key) ?? keyName(key);
  }
}

export const names = new Names();
