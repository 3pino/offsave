import { Shell } from '@3pino/app-kit-web/shell';

export type Tab = 'home' | 'rules' | 'friends' | 'events' | 'settings';
/** 設定の中のサブ画面 */
export type SettingsSub = 'protection' | 'android' | 'devices' | 'hidden';

/** 幅の境目（@3pino/design-system の決まり。767px 以下がモバイル） */
const MOBILE_QUERY = '(max-width: 767px)';

/** 画面の状態: トースト・確認（土台の Shell）、いまのタブ、戻る操作で閉じる層。 */
class Ui {
  readonly shell = new Shell();
  tab = $state<Tab>('home');
  isMobile = $state(typeof matchMedia === 'function' ? matchMedia(MOBILE_QUERY).matches : false);
  /** ルールの画面で、先頭に出す人（友達の画面の「ルールを編集」から来たとき） */
  rulesFocus = $state<string | null>(null);
  /** 設定で、開いて見せるサブ画面（ホームの警告から来たとき） */
  settingsSub = $state<SettingsSub | null>(null);
  /** 戻る操作で閉じる層（サブ画面）。新しいものが後ろ */
  // リアクティブにしない（$effect の中から pushLayer すると、読み書きで自分を何度も作り直してしまう）
  private layers: { close: () => void }[] = [];

  constructor() {
    if (typeof matchMedia === 'function') {
      matchMedia(MOBILE_QUERY).addEventListener('change', (e) => (this.isMobile = e.matches));
    }
  }

  toast(text: string, kind: 'info' | 'error' = 'info', action?: { label: string; run: () => void }, ms: number | null = 5000) {
    return this.shell.toast(text, kind, action, ms);
  }

  confirm = (opts: Parameters<Shell['confirm']>[0]) => this.shell.confirm(opts);

  selectTab(tab: Tab) {
    this.tab = tab;
    if (tab !== 'rules') this.rulesFocus = null;
    if (tab !== 'settings') this.settingsSub = null;
    // タブを替えたら、開いていたサブ画面は閉じる
    for (const l of [...this.layers].reverse()) l.close();
  }

  /** ルールの画面を、その人を先頭にして開く */
  openRulesOf(userId: string) {
    this.rulesFocus = userId;
    this.selectTab('rules');
  }

  /** 設定を、そのサブ画面を開いて開く */
  openSettings(sub: SettingsSub) {
    this.selectTab('settings');
    this.settingsSub = sub;
  }

  /**
   * サブ画面を開いている間、戻る操作（Android の戻る・Alt+←）で `close` を呼ぶ層を積む。返り値で外す。
   * `close` は画面を閉じる（閉じると呼び出し側が返り値の関数で層を外す）。
   */
  pushLayer(close: () => void): () => void {
    const entry = { close };
    this.layers = [...this.layers, entry];
    return () => {
      this.layers = this.layers.filter((l) => l !== entry);
    };
  }

  depth(): number {
    return this.layers.length;
  }

  /** 1 つ戻る。戻る先がなければ false（Android ではアプリが裏に回る） */
  back(): boolean {
    const top = this.layers.at(-1);
    if (!top) return false;
    top.close();
    return true;
  }
}

export const ui = new Ui();
