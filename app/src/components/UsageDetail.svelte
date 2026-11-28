<script lang="ts">
  import { untrack } from 'svelte';
  import Eye from '@lucide/svelte/icons/eye';
  import EyeOff from '@lucide/svelte/icons/eye-off';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import { duration } from '../lib/format';
  import { hidden } from '../lib/hidden.svelte';
  import { limitText, ruleDisplayName, ruleMatches } from '../lib/limits';
  import { names } from '../lib/names.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Rule, RuleInput, Target, UserStatus } from '../lib/types';
  import { isSite, ruleOfKey, sumSeconds, type UsageTarget } from '../lib/usage';
  import { usagePrefs } from '../lib/usagePrefs.svelte';
  import KeyIcon from './KeyIcon.svelte';
  import Page from './Page.svelte';
  import RuleEditor from './RuleEditor.svelte';
  import UsageStats from './UsageStats.svelte';

  // 利用の詳細: アプリ・サイト 1 つ、またはルール 1 つ（カテゴリ全体）の、今日の利用と棒グラフ。
  // ここから、その対象を入れたルールを、すぐに追加・編集できる（編集から戻ると、この画面に戻る）
  let {
    userId,
    ownerName,
    status,
    target,
    onback,
  }: {
    userId: string;
    /** ルールを受ける人の呼び方（「あなた」「bob さん」） */
    ownerName: string;
    status: UserStatus;
    target: UsageTarget;
    onback: () => void;
  } = $props();

  const api = $derived(session.api!);
  const rules = $derived(status.rules.map((r) => r.rule));

  // 開いたあとに、ルールが消えた（友達が削除した）ら閉じる
  const rule = $derived<Rule | null>(target.kind === 'rule' ? (rules.find((r) => r.id === target.id) ?? null) : ruleOfKey(rules, target.key));
  $effect(() => {
    if (target.kind === 'rule' && !rule) onback();
  });

  const key = $derived(target.kind === 'key' ? target.key : null);
  const name = $derived(key ? names.label(key) : rule ? ruleDisplayName(rule) : '');
  const title = $derived(key ? (isSite(key) ? 'サイトの利用' : 'アプリの利用') : 'ルールの利用');

  const pick = $derived<(k: string) => boolean>(
    key
      ? (k) => k === key
      : rule
        ? (k) => ruleMatches(rule, k) && (usagePrefs.showHidden || !hidden.has(k))
        : () => false,
  );

  // ルール全体のときは、中身（今日使ったもの）を並べる
  const members = $derived(target.kind === 'rule' ? status.usage_today.filter((u) => pick(u.key)) : []);
  const used = $derived(sumSeconds(status.usage_today, pick));

  // ルールの追加・編集
  let editing = $state.raw<{ rule: Rule | null; preset: Target[] } | null>(null);
  $effect(() => {
    if (!editing) return;
    return ui.pushLayer(() => (editing = null));
  });

  function openEditor() {
    editing = rule ? { rule, preset: [] } : { rule: null, preset: key ? [{ label: names.label(key), keys: [key] }] : [] };
  }

  async function save(input: RuleInput) {
    const e = editing!;
    if (e.rule) await api.updateRule(e.rule.id, input);
    else await api.createRule(userId, input);
    editing = null;
    ui.toast(e.rule ? 'ルールを変更しました' : 'ルールを追加しました');
  }

  const isHidden = $derived(key !== null && hidden.has(key));
  const candidates = $derived(untrack(() => names.candidates(status, ownerName === 'あなた')));
</script>

{#if editing}
  <RuleEditor
    {ownerName}
    initial={editing.rule}
    presetTargets={editing.preset}
    {candidates}
    otherRules={rules.filter((r) => r.id !== editing?.rule?.id)}
    onsave={save}
    oncancel={() => (editing = null)}
  />
{:else}
  <Page {title} {onback}>
    <div class="head row">
      {#if key}<KeyIcon {key} size={40} />{/if}
      <div class="grow">
        <div class="head-name">{name}</div>
        {#if key}
          <div class="muted small">{isSite(key) ? 'サイト' : 'アプリ'}{rule ? `・ルール「${ruleDisplayName(rule)}」` : ''}</div>
        {:else if rule}
          <div class="muted small">ルール{rule.enabled ? '' : '（無効）'}</div>
        {/if}
      </div>
    </div>

    <div class="row row-wrap">
      {#if rule}
        <Button icon={Pencil} onclick={openEditor}>ルールを編集</Button>
      {:else}
        <Button icon={Plus} onclick={openEditor}>ルールを追加</Button>
      {/if}
      {#if key}
        {#if isHidden}
          <Button icon={Eye} onclick={() => hidden.remove(key)}>再表示</Button>
        {:else}
          <Button icon={EyeOff} onclick={() => hidden.add(key)}>非表示</Button>
        {/if}
      {/if}
    </div>

    <UsageStats {userId} {status} {pick} />

    {#if target.kind === 'rule' && rule}
      <section class="block">
        <h2 class="block-title">制限</h2>
        <ul class="limit-list">
          {#each rule.limits as l, i (i)}
            <li>{limitText(l)}</li>
          {/each}
        </ul>
      </section>
      <section class="block">
        <h2 class="block-title">今日の利用（合計 {duration(used)}）</h2>
        {#each members as m (m.key)}
          <div class="row member">
            <KeyIcon key={m.key} size={24} />
            <span class="grow member-name">{names.label(m.key)}</span>
            <span class="muted">{duration(m.seconds)}</span>
          </div>
        {:else}
          <p class="empty">今日は、まだ使っていません。</p>
        {/each}
        <h2 class="block-title">対象</h2>
        <div class="muted small">{rule.targets.map((t) => t.label).join('、')}</div>
      </section>
    {/if}
  </Page>
{/if}

<style>
  .head-name {
    font-size: var(--fs-lg);
    font-weight: var(--fw-bold);
    overflow-wrap: anywhere;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  .member {
    min-height: 32px;
  }
  .member-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .limit-list {
    margin: 0;
    padding-left: var(--space-8);
    font-size: var(--fs-base);
  }
</style>
