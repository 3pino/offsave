<script lang="ts">
  import { addDays, ago, dateTime, duration, limitText } from './lib/format';
  import { names } from './lib/names.svelte';
  import type { Rule, RuleStatus, UserStatus } from './lib/types';

  let {
    status,
    /** 友達の画面（ルールを変えられる）か、自分の画面か */
    mine,
    onEdit,
    onDelete,
    onOverride,
    onDeleteOverride,
  }: {
    status: UserStatus;
    mine: boolean;
    onEdit?: (r: Rule) => void;
    onDelete?: (r: Rule) => void;
    onOverride?: (date: string, ruleId: string | null) => void;
    onDeleteOverride?: (id: string) => void;
  } = $props();

  const STATE: Record<string, [string, string]> = {
    online: ['接続中', 'ok'],
    suspect: ['連絡なし', 'bad'],
    offline: ['停止', 'warn'],
    new: ['未接続', 'muted'],
  };

  const pct = (r: RuleStatus) =>
    r.rule.limit.type === 'daily_total' && r.rule.limit.seconds > 0
      ? Math.min(100, (r.used_seconds / r.rule.limit.seconds) * 100)
      : 0;

  $effect(() => names.learn(status.labels));

  const tomorrow = $derived(addDays(status.date, 1));
  const unlocked = $derived(status.unlock_until > Date.now());
</script>

{#if unlocked}
  <div class="card warn">制限を一時解除中です（{dateTime(status.unlock_until)} まで）</div>
{/if}

<h2>デバイス</h2>
{#each status.devices as d}
  {@const s = STATE[d.state] ?? [d.state, 'muted']}
  <div class="card">
    <div class="row">
      <b class="grow">{d.name}</b>
      <span class="badge {s[1]}">{s[0]}</span>
    </div>
    <div class="muted">
      {d.platform} ・ 最後の連絡 {d.last_seen ? ago(d.last_seen) : 'なし'}
      {#if d.foreground}・ いま {names.label(d.foreground)}{/if}
    </div>
    {#if d.state !== 'new' && !(d.flags.accessibility && d.flags.device_admin)}
      <div class="bad">
        保護が不完全です:
        {#if !d.flags.accessibility}Accessibility が無効 {/if}
        {#if !d.flags.device_admin}端末管理者が無効{/if}
      </div>
    {/if}
  </div>
{:else}
  <p class="muted">デバイスがありません。</p>
{/each}

<h2>今日の利用（{status.date}）</h2>
<div class="card">
  {#each status.usage_today.slice(0, 12) as u}
    <div class="row">
      <div class="grow">{names.label(u.key)}</div>
      <div class="muted">{duration(u.seconds)}</div>
    </div>
  {:else}
    <span class="muted">まだ記録がありません。</span>
  {/each}
</div>

<h2>ルール</h2>
{#each status.rules as r}
  <div class="card">
    <div class="row">
      <b class="grow">{r.rule.name}{r.rule.enabled ? '' : '（無効）'}</b>
      {#if !mine}
        <button class="small" onclick={() => onEdit?.(r.rule)}>編集</button>
        <button class="small danger" onclick={() => onDelete?.(r.rule)}>削除</button>
      {/if}
    </div>
    <div class="muted">{r.rule.targets.map((t) => t.label).join('、')}</div>
    <div>{limitText(r.rule.limit)}</div>
    {#if r.rule.limit.type === 'daily_total'}
      <div class="bar" class:full={pct(r) >= 100}><div style="width: {pct(r)}%"></div></div>
      <div class="muted">今日 {duration(r.used_seconds)} / {duration(r.rule.limit.seconds)}</div>
    {:else}
      <div class="muted">今日 {duration(r.used_seconds)} 使用</div>
    {/if}
    <div class="muted">決めた人: {r.author.display_name}</div>
  </div>
{:else}
  <p class="muted">ルールはまだありません。</p>
{/each}

{#if !mine}
  <h2>制限を外す日</h2>
  <div class="row wrap">
    <button onclick={() => onOverride?.(status.date, null)}>今日は制限なし</button>
    <button onclick={() => onOverride?.(tomorrow, null)}>明日は制限なし</button>
  </div>
{/if}
{#if status.overrides.length > 0}
  <div style="margin-top: 8px">
    {#each status.overrides as o}
      <div class="row card" style="padding: 8px 12px">
        <div class="grow">{o.date} は {o.rule_id ? (status.rules.find((r) => r.rule.id === o.rule_id)?.rule.name ?? '一部のルール') : 'すべての制限'}を外す</div>
        {#if !mine}<button class="small" onclick={() => onDeleteOverride?.(o.id)}>取り消す</button>{/if}
      </div>
    {/each}
  </div>
{/if}
