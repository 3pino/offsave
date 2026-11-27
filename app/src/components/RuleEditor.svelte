<script lang="ts">
  import { untrack } from 'svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import TextInput from '@3pino/ui-kit-web/TextInput.svelte';
  import Toggle from '@3pino/ui-kit-web/Toggle.svelte';
  import { errorText } from '../lib/errors';
  import { keyName } from '../lib/format';
  import { limitError, newLimit } from '../lib/limits';
  import { isAndroid } from '../lib/native';
  import { names } from '../lib/names.svelte';
  import type { Limit, Rule, RuleInput, Target, UsageItem } from '../lib/types';
  import LimitEditor from './LimitEditor.svelte';
  import Page from './Page.svelte';

  let {
    ownerName,
    initial = null,
    usage = [],
    onsave,
    oncancel,
  }: {
    /** ルールを受ける人の呼び方（「あなた」「bob さん」など。見出しに出す） */
    ownerName: string;
    initial?: Rule | null;
    /** 対象の人が今日使ったもの（ここから選べる） */
    usage?: UsageItem[];
    onsave: (rule: RuleInput) => Promise<void>;
    oncancel: () => void;
  } = $props();

  // 編集の開始時の値だけを使う（あとから props が変わっても、入力中の内容は消さない）
  const init = untrack(() => initial);

  let name = $state(init?.name ?? '');
  let enabled = $state(init?.enabled ?? true);
  let targets = $state<Target[]>(init ? init.targets.map((t) => ({ label: t.label, keys: [...t.keys] })) : []);
  let limits = $state<Limit[]>(init ? init.limits.map((l) => ({ ...l })) : [newLimit('daily_total')]);

  let manualKey = $state('');
  let manualLabel = $state('');
  let search = $state('');
  let picking = $state(false);
  let busy = $state(false);
  let error = $state('');

  const has = (key: string) => targets.some((t) => t.keys.includes(key));

  function addTarget(label: string, key: string) {
    if (!has(key)) targets.push({ label, keys: [key] });
  }

  function addManual() {
    let key = manualKey.trim();
    if (!key) return;
    // 接頭辞がなければ、ドメインとして扱う（URL が貼られたときは、ホスト名だけにする）
    if (!/^(android|win|linux|domain):/.test(key)) {
      key = `domain:${key.replace(/^[a-z]+:\/\//i, '').split(/[/?#]/)[0].replace(/^www\./i, '').toLowerCase()}`;
    }
    addTarget(manualLabel.trim() || keyName(key), key);
    manualKey = '';
    manualLabel = '';
  }

  const filtered = $derived(
    names.apps.filter((a) => (a.label + a.package).toLowerCase().includes(search.toLowerCase())).slice(0, 40),
  );

  async function save() {
    error = '';
    if (!name.trim()) return (error = 'ルールの名前を入力してください');
    if (targets.length === 0) return (error = '対象のアプリかサイトを 1 つ以上選んでください');
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

<Page title={init ? 'ルールを編集' : 'ルールを追加'} onback={oncancel}>
  {#snippet actions()}
    <Button variant="primary" disabled={busy} onclick={save}>{busy ? '保存中…' : '保存'}</Button>
  {/snippet}

  <p class="muted small editor-for">{ownerName}のルールです。</p>

  <label class="field">
    <span class="field-label">ルールの名前</span>
    <TextInput bind:value={name} placeholder="例: ゲーム" />
  </label>

  <section class="editor-block">
    <h2 class="block-title">対象（合計で数えます）</h2>
    <div class="chips">
      {#each targets as t, i (t.keys.join())}
        <span class="chip chip-on target">
          {t.label}
          <button type="button" class="target-x" aria-label="{t.label} を外す" onclick={() => targets.splice(i, 1)}><X size={14} /></button>
        </span>
      {:else}
        <span class="empty">まだありません。</span>
      {/each}
    </div>

    {#if usage.length > 0}
      <div class="muted small">今日使ったもの（押すと追加）</div>
      <div class="chips">
        {#each usage as u (u.key)}
          <button type="button" class="chip" class:chip-on={has(u.key)} onclick={() => addTarget(names.label(u.key), u.key)}>
            {has(u.key) ? '✓ ' : '＋ '}{names.label(u.key)}
          </button>
        {/each}
      </div>
    {/if}

    {#if isAndroid}
      <div><Button onclick={() => (picking = !picking)}>このデバイスのアプリから選ぶ</Button></div>
      {#if picking}
        <TextInput bind:value={search} placeholder="検索" />
        <div class="apps scroll">
          {#each filtered as a (a.package)}
            <div class="row app-row">
              <div class="grow">{a.label} <span class="muted small">{a.package}</span></div>
              <Button disabled={has(`android:${a.package}`)} onclick={() => addTarget(a.label, `android:${a.package}`)}>
                {has(`android:${a.package}`) ? '追加済み' : '追加'}
              </Button>
            </div>
          {/each}
        </div>
      {/if}
    {/if}

    <div class="field">
      <span class="field-label">サイトや別の OS のアプリを追加</span>
      <div class="row row-wrap">
        <div class="grow"><TextInput bind:value={manualKey} placeholder="youtube.com / android:com.x / win:game.exe" autocapitalize="off" /></div>
        <div class="manual-label"><TextInput bind:value={manualLabel} placeholder="名前（省略可）" /></div>
        <Button onclick={addManual}>追加</Button>
      </div>
    </div>
  </section>

  <section class="editor-block">
    <h2 class="block-title">制限（どれか 1 つでも当たると使えません）</h2>
    {#each limits as l, i (i)}
      <LimitEditor limit={l} onchange={(next) => (limits[i] = next)} onremove={limits.length > 1 ? () => limits.splice(i, 1) : undefined} />
    {/each}
    <div class="row row-wrap">
      <Button icon={Plus} onclick={() => limits.push(newLimit('daily_total'))}>1日の合計を足す</Button>
      <Button icon={Plus} onclick={() => limits.push(newLimit('window'))}>使えない時間帯を足す</Button>
    </div>
    <p class="muted small">例: 合計 1 時間 ＋ 毎日 6:00–7:00 は禁止 ＋ 平日は 15:00–23:00 も禁止。</p>
  </section>

  <SettingRow name="有効にする" description="外すと、このルールは働きません。">
    {#snippet control()}
      <div class="setting-row-control-slot"><Toggle bind:checked={enabled} label="有効にする" /></div>
    {/snippet}
  </SettingRow>

  {#if error}<p class="error-text">{error}</p>{/if}
  <div class="row">
    <Button variant="primary" disabled={busy} onclick={save}>{busy ? '保存中…' : '保存'}</Button>
    <Button onclick={oncancel}>キャンセル</Button>
  </div>
</Page>

<style>
  .editor-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .editor-block p,
  .editor-for {
    margin: 0;
  }
  .target {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding-right: var(--space-3);
  }
  .target-x {
    display: inline-flex;
    border: none;
    background: transparent;
    color: inherit;
    padding: var(--space-1);
    border-radius: var(--radius-pill);
  }
  .apps {
    max-height: 220px;
  }
  .app-row {
    padding: var(--space-2) 0;
  }
  .manual-label {
    width: 150px;
  }
</style>
