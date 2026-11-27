<script lang="ts">
  import { onMount } from 'svelte';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Button from '@3pino/ui-kit-web/Button.svelte';
  import IconButton from '@3pino/ui-kit-web/IconButton.svelte';
  import { errorText } from '../lib/errors';
  import { addDays, duration } from '../lib/format';
  import { limitText, todayTotal, weekdayOfDate } from '../lib/limits';
  import { names } from '../lib/names.svelte';
  import { session } from '../lib/session.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Rule, RuleInput, RuleStatus, User, UserStatus } from '../lib/types';
  import Page from './Page.svelte';
  import RuleEditor from './RuleEditor.svelte';

  // ルールの画面。自分のルール（友達が決めたもの）と、友達のルール（自分が決められる）を、人ごとに並べる

  interface Person {
    user: User;
    mine: boolean;
    status: UserStatus | null;
  }

  const saved = $derived(session.saved!);
  const api = $derived(session.api!);

  let people = $state.raw<Person[]>([]);
  // 編集中のルール。候補（対象に選べるアプリ・サイト）は、開いたときに 1 回だけ作る
  let editing = $state.raw<{ person: Person; rule: Rule | null; candidates: { key: string; label: string }[] } | null>(null);

  /** その人の対象に選べるもの: 自分ならこのデバイスのアプリ、それに加えて、その人が使ったことのあるもの */
  function candidatesOf(p: Person): { key: string; label: string }[] {
    const map = new Map<string, string>();
    if (p.mine) for (const a of names.apps) map.set(`android:${a.package}`, a.label);
    for (const l of p.status?.labels ?? []) if (!map.has(l.key)) map.set(l.key, l.label);
    for (const u of p.status?.usage_today ?? []) if (!map.has(u.key)) map.set(u.key, names.label(u.key));
    return [...map].map(([key, label]) => ({ key, label }));
  }

  function openEditor(person: Person, rule: Rule | null) {
    editing = { person, rule, candidates: candidatesOf(person) };
  }

  // 戻る操作（Android の戻る）で、編集を閉じる
  $effect(() => {
    if (!editing) return;
    return ui.pushLayer(() => (editing = null));
  });

  async function refresh() {
    // 編集中は取り直さない（入力中の画面を重くしない）
    if (editing) return;
    try {
      const friends = (await api.friends()).filter((f) => f.state === 'accepted');
      const list: { user: User; mine: boolean }[] = [
        { user: { id: saved.userId, login_name: '', display_name: 'あなた' }, mine: true },
        ...friends.map((f) => ({ user: f.user, mine: false })),
      ];
      const statuses = await Promise.all(list.map((p) => api.status(p.user.id).catch(() => null)));
      people = list.map((p, i) => ({ ...p, status: statuses[i] }));
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }

  onMount(() => {
    void refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });

  // 友達の画面から来たときは、その人を先頭に出す
  const ordered = $derived(
    ui.rulesFocus ? [...people].sort((a, b) => Number(b.user.id === ui.rulesFocus) - Number(a.user.id === ui.rulesFocus)) : people,
  );

  const pct = (used: number, total: number) => (total > 0 ? Math.min(100, (used / total) * 100) : 100);

  async function save(input: RuleInput) {
    const e = editing!;
    if (e.rule) await api.updateRule(e.rule.id, input);
    else await api.createRule(e.person.user.id, input);
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
    const ok = await ui.confirm({ title: 'ルールを削除しますか？', body: `「${r.name}」を削除します。`, okLabel: '削除する', danger: true });
    if (ok) await act(() => api.deleteRule(r.id));
  }

  const ruleName = (s: UserStatus, id: string | null) =>
    id ? (s.rules.find((r) => r.rule.id === id)?.rule.name ?? '一部のルール') : 'すべての制限';
</script>

{#snippet ruleCard(person: Person, status: UserStatus, rs: RuleStatus)}
  {@const rule = rs.rule}
  {@const total = todayTotal(rule.limits, weekdayOfDate(status.date))}
  <div class="card">
    <div class="card-title">
      <span class="grow">{rule.name}{rule.enabled ? '' : '（無効）'}</span>
      {#if !person.mine}
        <IconButton label="編集" icon={Pencil} onclick={() => openEditor(person, rule)} />
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
    <div class="muted small">決めた人: {rs.author.display_name}</div>
  </div>
{/snippet}

{#if editing}
  <RuleEditor
    ownerName={editing.person.mine ? 'あなた' : `${editing.person.user.display_name} さん`}
    initial={editing.rule}
    candidates={editing.candidates}
    onsave={save}
    oncancel={() => (editing = null)}
  />
{:else}
  <Page title="ルール">
    {#each ordered as person (person.user.id)}
      {@const status = person.status}
      <section class="person">
        <h2 class="block-title">{person.mine ? 'あなたのルール' : `${person.user.display_name} さんのルール`}</h2>
        {#if person.mine}
          <p class="muted small">あなたのルールは、友達が決めます。自分で足せるのは、制限を厳しくするルールだけです（あとから外すには友達の操作が必要です）。</p>
        {/if}

        {#if status}
          {#each status.rules as rs (rs.rule.id)}
            {@render ruleCard(person, status, rs)}
          {:else}
            <p class="empty">ルールはまだありません。</p>
          {/each}
          <div><Button icon={Plus} onclick={() => openEditor(person, null)}>ルールを追加</Button></div>

          {#if !person.mine || status.overrides.length > 0}
            <h3 class="block-title">制限を外す日</h3>
          {/if}
          {#if !person.mine}
            <div class="row row-wrap">
              <Button onclick={() => act(() => api.createOverride(person.user.id, status.date, null))}>今日は制限なし</Button>
              <Button onclick={() => act(() => api.createOverride(person.user.id, addDays(status.date, 1), null))}>明日は制限なし</Button>
            </div>
          {/if}
          {#each status.overrides as o (o.id)}
            <div class="card">
              <div class="row">
                <div class="grow">{o.date} は {ruleName(status, o.rule_id)}を外す</div>
                {#if !person.mine}<Button onclick={() => act(() => api.deleteOverride(o.id))}>取り消す</Button>{/if}
              </div>
            </div>
          {/each}
        {:else}
          <p class="empty">読み込み中…</p>
        {/if}
      </section>
    {:else}
      <p class="empty">読み込み中…</p>
    {/each}
  </Page>
{/if}

<style>
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
