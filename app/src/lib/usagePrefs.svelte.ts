import { getItem, setItem } from '@3pino/app-kit-web/core/storage';

/**
 * 利用の表示の設定（このデバイスの画面だけ。記録・制限には関係しない）。
 * - showHidden: 「非表示のアプリ」も、一覧と合計に含める（既定: 含めない）
 * - byRule: ルールごとにまとめて表示する（既定: まとめる）
 * - chart: 棒グラフの単位。`day` = 1 日ごと（週）、`hour` = 1 時間ごと（1 日）
 */
const KEY = 'offsave.usagePrefs';

export type ChartMode = 'day' | 'hour';

interface Stored {
  showHidden: boolean;
  byRule: boolean;
  chart: ChartMode;
}

const DEFAULTS: Stored = { showHidden: false, byRule: true, chart: 'day' };

function load(): Stored {
  try {
    const v = getItem(KEY);
    if (v) {
      const p = JSON.parse(v) as Partial<Stored>;
      return {
        showHidden: typeof p.showHidden === 'boolean' ? p.showHidden : DEFAULTS.showHidden,
        byRule: typeof p.byRule === 'boolean' ? p.byRule : DEFAULTS.byRule,
        chart: p.chart === 'hour' ? 'hour' : 'day',
      };
    }
  } catch {
    /* 既定値にする */
  }
  return { ...DEFAULTS };
}

class UsagePrefs {
  private v = $state<Stored>(load());
  /** ホームで見ている人（null = 自分）。保存しない（アプリを開き直すと自分に戻る） */
  viewing = $state.raw<{ id: string; name: string } | null>(null);

  get showHidden() {
    return this.v.showHidden;
  }
  get byRule() {
    return this.v.byRule;
  }
  get chart() {
    return this.v.chart;
  }

  set showHidden(x: boolean) {
    this.save({ showHidden: x });
  }
  set byRule(x: boolean) {
    this.save({ byRule: x });
  }
  set chart(x: ChartMode) {
    this.save({ chart: x });
  }

  private save(patch: Partial<Stored>) {
    this.v = { ...this.v, ...patch };
    setItem(KEY, JSON.stringify(this.v));
  }
}

export const usagePrefs = new UsagePrefs();
