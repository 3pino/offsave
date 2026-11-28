<script lang="ts">
  import { untrack } from 'svelte';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import { errorText } from '../lib/errors';
  import { addDays, duration } from '../lib/format';
  import { limitText, ruleDisplayName, todayTotal, weekdayOfDate } from '../lib/limits';
  import { names } from '../lib/names.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import { viewing } from '../lib/viewing.svelte';
  import type { Rule, RuleInput, RuleStatus, UserStatus } from '../lib/types';
  import KeyIcon from './KeyIcon.svelte';
  import Page from './Page.svelte';
  import PersonHeading from './PersonHeading.svelte';
  import RuleEditor from './RuleEditor.svelte';
  import TargetPicker from './TargetPicker.svelte';

  // ルールの画面。見出しで選んだ人（自分、または承認済みの友達）のルールを出す。
  // 自分のルールは友達が決めたもの、友達のルールは自分が決められる

  const api = $derived(session.api!);
  const userId = $derived(viewing.id);
  const mine = $derived(viewing.mine);
  const ownerName = $derived(mine ? 'あなた' : viewing.name);

  let status = $state.raw<UserStatus | null>(null);
  // 編集中のルール。候補（対象に選べるアプリ・サイト）は、開いたときに 1 回だけ作る
  let editing = $state.raw<{ rule: Rule | null; candidates: { key: string; label: string }[] } | null>(null);

  /** 対象に選べるもの: 自分ならこのデバイスのアプリ、それに加えて、その人が使ったことのあるもの */
  const candidates = () => names.candidates(status, mine);

  // 「監視を一時停止するアプリ」を選んでいるところ（友達だけが決められる）
  let pausePicking = $state(false);
  const setPause = (keys: string[]) => act(() => api.setPauseApps(userId, keys));

  function openEditor(rule: Rule | null) {
    editing = { rule, candidates: candidates() };
  }

  // 戻る操作（Android の戻る）で、編集を閉じる
  $effect(() => {
    if (!editing) return;
    return ui.pushLayer(() => (editing = null));
  });

  async function refresh() {
    // 編集中は取り直さない（入力中の画面を重くしない）
    if (editing) return;
    const id = userId;
    try {
      const s = await api.status(id);
      // 取っている間に見る人が替わっていたら、捨てる
      if (id !== userId) return;
      names.learn(s.labels);
      status = s;
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  // 見る人を変えたら、前の人の表示は捨てて取り直す。そのあとは 5 秒ごと
  $effect(() => {
    void userId;
    untrack(() => {
      status = null;
      pausePicking = false;
      void refresh();
    });
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });

  const pct = (used: number, total: number) => (total > 0 ? Math.min(100, (used / total) * 100) : 100);

  async function save(input: RuleInput) {
    const e = editing!;
    if (e.rule) await api.updateRule(e.rule.id, input);
    else await api.createRule(userId, input);
    editing = null;
    await refresh();
  }

  async function act(f: () => Promise<unknown>) {
    try {
      await f();
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
    await refresh();
  }

  async function remove(r: Rule) {
    const ok = await ui.confirm({ title: 'ルールを削除しますか？', body: `「${ruleDisplayName(r)}」を削除します。`, okLabel: '削除する', danger: true });
    if (ok) await act(() => api.deleteRule(r.id));
  }

  function ruleName(s: UserStatus, id: string | null): string {
    if (!id) return 'すべての制限';
    const found = s.rules.find((r) => r.rule.id === id);
    return found ? ruleDisplayName(found.rule) : '一部のルール';
  }
</script>

{#snippet ruleCard(status: UserStatus, rs: RuleStatus)}
  {@const rule = rs.rule}
  {@const total = todayTotal(rule.limits, weekdayOfDate(status.date))}
  <div class="card">
    <div class="card-title">
      <span class="grow">{ruleDisplayName(rule)}{rule.enabled ? '' : '（無効）'}</span>
      <!-- 自分のルールも、緩めない変更はできる。削除は友達だけ -->
      <IconButton label="編集" icon={Pencil} onclick={() => openEditor(rule)} />
      {#if !mine}
        <IconButton label="削除" icon={Trash2} onclick={() => remove(rule)} />
      {/if}
    </div>
    <div class="muted small">{rule.targets.map((t) => t.label).join('、')}</div>
    <ul class="limit-list">
      {#each rule.limits as l}
        <li>{limitText(l)}</li>
      {/each}
    </ul>
    {#if total !== null}
      {@const p = pct(rs.used_seconds, total)}
      <div class="bar" class:bar-full={p >= 100}><div style="width: {p}%"></div></div>
      <div class="muted small">今日 {duration(rs.used_seconds)} / {duration(total)}</div>
    {:else}
      <div class="muted small">今日 {duration(rs.used_seconds)} 使用</div>
    {/if}
  </div>
{/snippet}

{#snippet heading()}
  <PersonHeading />
{/snippet}

{#if editing}
  <RuleEditor
    {ownerName}
    initial={editing.rule}
    candidates={editing.candidates}
    otherRules={(status?.rules ?? []).map((r) => r.rule).filter((r) => r.id !== editing?.rule?.id)}
    onsave={save}
    oncancel={() => (editing = null)}
  />
{:else}
  <Page title="{ownerName}のルール" {heading}>
    <section class="person">
      {#if status}
        {#each status.rules as rs (rs.rule.id)}
          {@render ruleCard(status, rs)}
        {:else}
          <p class="empty">ルールはまだありません。</p>
        {/each}
        <div><Button icon={Plus} onclick={() => openEditor(null)}>ルールを追加</Button></div>

        {#if !mine || status.overrides.length > 0}
          <h3 class="block-title">制限を外す日</h3>
        {/if}
        {#if !mine}
          <div class="row row-wrap">
            <Button onclick={() => act(() => api.createOverride(userId, status!.date, null))}>今日は制限なし</Button>
            <Button onclick={() => act(() => api.createOverride(userId, addDays(status!.date, 1), null))}>明日は制限なし</Button>
          </div>
        {/if}
        {#each status.overrides as o (o.id)}
          <div class="card">
            <div class="row">
              <div class="grow">{o.date} は {ruleName(status, o.rule_id)}を外す</div>
              {#if !mine}<Button onclick={() => act(() => api.deleteOverride(o.id))}>取り消す</Button>{/if}
            </div>
          </div>
        {/each}

        {#if !mine || status.pause_apps.length > 0}
          <h3 class="block-title">監視を一時停止するアプリ</h3>
          <p class="muted small">決済アプリなど、Accessibility が有効だと使えないアプリ。開いている間だけ、監視を自動で止めます。</p>
          {#each status.pause_apps as key (key)}
            <div class="row pause-row">
              <KeyIcon {key} size={28} />
              <span class="grow">{names.label(key)}</span>
              {#if !mine}
                <IconButton label="{names.label(key)} を外す" icon={Trash2} onclick={() => setPause(status!.pause_apps.filter((k) => k !== key))} />
              {/if}
            </div>
          {/each}
          {#if !mine}
            <div><Button icon={Plus} onclick={() => (pausePicking = true)}>アプリを追加</Button></div>
          {/if}
        {/if}
      {:else}
        <p class="empty">読み込み中…</p>
      {/if}
    </section>
  </Page>
{/if}

{#if pausePicking && status}
  <TargetPicker
    appsOnly
    title="監視を一時停止するアプリ"
    candidates={candidates().filter((c) => c.key.startsWith('android:'))}
    picked={(k) => (status?.pause_apps ?? []).includes(k)}
    onpick={(k) => setPause([...(status?.pause_apps ?? []), k])}
    onunpick={(k) => setPause((status?.pause_apps ?? []).filter((x) => x !== k))}
    onclose={() => (pausePicking = false)}
  />
{/if}

<style>
  .pause-row {
    min-height: 32px;
  }
  .person {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .person p {
    margin: 0;
  }
  .limit-list {
    margin: 0;
    padding-left: var(--space-8);
    font-size: var(--fs-base);
  }
</style>
