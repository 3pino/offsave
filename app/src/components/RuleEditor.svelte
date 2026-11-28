<script lang="ts">
  import { untrack } from 'svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import Toggle from '@3pino/ui-kit-web/Toggle.svelte';
  import { errorText } from '../lib/errors';
  import { keysOverlap, limitError, newLimit, ruleDisplayName } from '../lib/limits';
  import { ui } from '../lib/ui.svelte';
  import type { Limit, Rule, RuleInput, Target } from '../lib/types';
  import KeyIcon from './KeyIcon.svelte';
  import LimitEditor from './LimitEditor.svelte';
  import Page from './Page.svelte';
  import TargetPicker from './TargetPicker.svelte';

  // ルールの追加・編集。2 つの画面に分ける: ① 名前と対象 → ② 制限
  let {
    ownerName,
    initial = null,
    candidates = [],
    otherRules = [],
    presetTargets = [],
    onsave,
    oncancel,
  }: {
    /** ルールを受ける人の呼び方（「あなた」「bob さん」など。見出しに出す） */
    ownerName: string;
    initial?: Rule | null;
    /** 対象に選べるアプリ・サイト（ポップアップに出す） */
    candidates?: { key: string; label: string }[];
    /** 同じ人のほかのルール（1 つのアプリ・サイトは 1 つのルールにしか入れられない） */
    otherRules?: Rule[];
    /** 新しいルールのとき、あらかじめ対象に入れておくもの */
    presetTargets?: Target[];
    onsave: (rule: RuleInput) => Promise<void>;
    oncancel: () => void;
  } = $props();

  // 編集の開始時の値だけを使う（あとから props が変わっても、入力中の内容は消さない）
  const init = untrack(() => initial);

  /** 自分のルールを直すとき（厳しくする変更だけ保存できる） */
  const selfEdit = untrack(() => ownerName === 'あなた') && init !== null;

  let step = $state<1 | 2>(1);
  let name = $state(init?.name ?? '');
  let enabled = $state(init?.enabled ?? true);
  let targets = $state.raw<Target[]>(
    (init ? init.targets : untrack(() => presetTargets)).map((t) => ({ label: t.label, keys: [...t.keys] })),
  );
  let limits = $state<Limit[]>(init ? init.limits.map((l) => ({ ...l })) : [newLimit('daily_total')]);

  let picking = $state(false);
  let busy = $state(false);
  let error = $state('');

  // ② から戻る操作（Android の戻る）で、① に戻る
  $effect(() => {
    if (step !== 2) return;
    return ui.pushLayer(() => (step = 1));
  });

  const has = (key: string) => targets.some((t) => t.keys.includes(key));

  /** ほかのルールにすでに入っているなら、そのルールの名前 */
  function blockedBy(key: string): string | null {
    const other = otherRules.find((r) => r.targets.some((t) => t.keys.some((k) => keysOverlap(k, key))));
    return other ? ruleDisplayName(other) : null;
  }

  function addTarget(key: string, label: string) {
    if (!has(key) && !blockedBy(key)) targets = [...targets, { label, keys: [key] }];
  }

  function removeTarget(key: string) {
    targets = targets.filter((t) => !t.keys.includes(key));
  }

  function next() {
    error = '';
    if (targets.length === 0) return (error = '対象のアプリかサイトを 1 つ以上選んでください');
    step = 2;
  }

  async function save() {
    error = '';
    if (limits.length === 0) return (error = '制限を 1 つ以上入れてください');
    for (const l of limits) {
      const e = limitError(l);
      if (e) return (error = e);
    }
    busy = true;
    try {
      await onsave({ name: name.trim(), targets, limits, enabled });
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<Page title={(init ? 'ルールを編集' : 'ルールを追加') + (ownerName === 'あなた' ? '' : `（${ownerName}）`)} onback={step === 2 ? () => (step = 1) : oncancel}>
  {#snippet actions()}
    {#if step === 1}
      <Button variant="primary" onclick={next}>次へ</Button>
    {:else}
      <Button variant="primary" disabled={busy} onclick={save}>{busy ? '保存中…' : '保存'}</Button>
    {/if}
  {/snippet}

  {#if step === 1}
    <label class="field">
      <span class="field-label">ルールの名前（任意）</span>
      <TextInput bind:value={name} placeholder="例: ゲーム" />
      <span class="muted small">空欄なら、「YouTube」「YouTube ほか3件」のように、対象から名前を付けます。</span>
    </label>
    {#if selfEdit}
      <p class="muted small">自分でできるのは、厳しくする変更だけです（名前の変更・対象や制限の追加・曜日の追加・1 日の合計時間の短縮・使えない時間帯の拡大）。緩める変更は友達に頼んでください。</p>
    {/if}

    <section class="editor-block">
      <h2 class="block-title">対象（合計で数えます）</h2>
      <div class="targets">
        {#each targets as t (t.keys.join())}
          <div class="target-row">
            <KeyIcon key={t.keys[0]} size={28} />
            <span class="grow target-name">{t.label}</span>
            <button type="button" class="target-x" aria-label="{t.label} を外す" onclick={() => removeTarget(t.keys[0])}><X size={16} /></button>
          </div>
        {:else}
          <span class="empty">まだありません。</span>
        {/each}
      </div>
      <div><Button icon={Plus} onclick={() => (picking = true)}>アプリ・サイトを追加</Button></div>
    </section>

    {#if error}<p class="error-text">{error}</p>{/if}
  {:else}
    <div class="card summary">
      <div class="card-title">{ruleDisplayName({ name, targets })}</div>
      <div class="muted small">{targets.map((t) => t.label).join('、')}</div>
    </div>

    <section class="editor-block">
      <h2 class="block-title">制限（どれか 1 つでも当たると使えません）</h2>
      {#each limits as l, i (i)}
        <LimitEditor limit={l} onchange={(n) => (limits[i] = n)} onremove={limits.length > 1 ? () => limits.splice(i, 1) : undefined} />
      {/each}
      <div class="row row-wrap">
        <Button icon={Plus} onclick={() => limits.push(newLimit('daily_total'))}>1日の合計を足す</Button>
        <Button icon={Plus} onclick={() => limits.push(newLimit('window'))}>使えない時間帯を足す</Button>
      </div>
    </section>

    <SettingRow name="有効にする" description="外すと、このルールは働きません。">
      {#snippet control()}
        <div class="setting-row-control-slot"><Toggle bind:checked={enabled} label="有効にする" /></div>
      {/snippet}
    </SettingRow>

    {#if error}<p class="error-text">{error}</p>{/if}
  {/if}
</Page>

{#if picking}
  <TargetPicker {candidates} picked={has} blocked={blockedBy} onpick={addTarget} onunpick={removeTarget} onclose={() => (picking = false)} />
{/if}

<style>
  .editor-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .targets {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .target-row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-5);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .target-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .target-x {
    display: inline-flex;
    border: none;
    background: transparent;
    color: inherit;
    padding: var(--space-2);
    border-radius: var(--radius-pill);
  }
  .summary {
    gap: var(--space-2);
  }
</style>
