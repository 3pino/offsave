<script lang="ts" module>
  export interface Bar {
    /** 棒の下に出す文字（空なら出さない） */
    label: string;
    /** 秒 */
    value: number;
    /** 読み上げ・ツールチップ用の説明（「11/25（水） 1時間 5分」など） */
    title: string;
    /** まだ来ていない日・時間（薄く出す） */
    future?: boolean;
    /** 今日・今の時間（強調して出す） */
    current?: boolean;
  }
</script>

<script lang="ts">
  // 棒グラフ。棒（と、その下の文字）を押すと選べる。選んだ棒の数字は、親が上に出す
  let { bars, selected, onselect }: { bars: Bar[]; selected: number | null; onselect: (i: number) => void } = $props();

  // 一番長い棒を基準にする。短すぎる棒も、押せるように最低の高さを持たせる
  const max = $derived(Math.max(60, ...bars.map((b) => b.value)));
  const height = (b: Bar) => (b.value > 0 ? Math.max(4, Math.round((b.value / max) * 100)) : 0);
</script>

<div class="chart" class:chart-dense={bars.length > 12} role="group" aria-label="利用時間の棒グラフ">
  {#each bars as b, i (i)}
    <button
      type="button"
      class="col"
      class:selected={selected === i}
      class:future={b.future}
      class:current={b.current}
      aria-label={b.title}
      aria-pressed={selected === i}
      title={b.title}
      onclick={() => onselect(i)}
    >
      <span class="track"><span class="fill" style:height="{height(b)}%"></span></span>
      <span class="label">{b.label}</span>
    </button>
  {/each}
</div>

<style>
  .chart {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .chart-dense {
    gap: 2px;
  }
  .col {
    flex: 1;
    min-width: 0;
    /* ボタンを flex にしない（古い WebView では、中の span が潰れて棒が描かれない）。中はどれも block にする */
    display: block;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    cursor: pointer;
  }
  .track {
    display: block;
    position: relative;
    height: 110px;
    border-radius: var(--radius-xs);
    background: var(--border);
    overflow: hidden;
  }
  .fill {
    display: block;
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--accent);
    opacity: 0.55;
    border-radius: var(--radius-xs) var(--radius-xs) 0 0;
  }
  .current .fill,
  .selected .fill {
    opacity: 1;
  }
  .future .track {
    opacity: 0.4;
  }
  .selected .track {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .selected {
    color: var(--text);
    font-weight: var(--fw-bold);
  }
  .label {
    display: block;
    margin-top: var(--space-2);
    height: 1.2em;
    text-align: center;
    white-space: nowrap;
    overflow: visible;
  }
</style>
