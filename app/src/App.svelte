<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Bell from '@lucide/svelte/icons/bell';
  import House from '@lucide/svelte/icons/house';
  import ListChecks from '@lucide/svelte/icons/list-checks';
  import Settings from '@lucide/svelte/icons/settings';
  import Users from '@lucide/svelte/icons/users';
  import NavBar, { type NavBarItem } from '@3pino/ui-kit-web/NavBar.svelte';
  import { isBackKey, startBackNav } from '@3pino/app-kit-web/core/backNav';
  import { startContextMenuGuard } from '@3pino/app-kit-web/core/contextMenu';
  import { externalLinkHref } from '@3pino/app-kit-web/core/externalLink';
  import { isAndroid, isTauri } from '@3pino/app-kit-web/core/platform';
  import { startViewportSync, startZoomGuard } from '@3pino/app-kit-web/core/viewport';
  import ConfirmDialog from '@3pino/app-kit-web/shell/ConfirmDialog.svelte';
  import Toasts from '@3pino/app-kit-web/shell/Toasts.svelte';
  import { showUpdateBadge, UPDATE_BADGE_LABEL } from '@3pino/app-kit-web/updater';
  import Events from './components/Events.svelte';
  import Friends from './components/Friends.svelte';
  import Home from './components/Home.svelte';
  import Rules from './components/Rules.svelte';
  import Settings_ from './components/Settings.svelte';
  import Setup from './components/Setup.svelte';
  import { events } from './lib/events.svelte';
  import { names } from './lib/names.svelte';
  import { native } from './lib/native';
  import { protection } from './lib/protection.svelte';
  import { session } from './lib/session.svelte';
  import { ui, type Tab } from './lib/ui.svelte';
  import { updater } from './lib/updater.svelte';
  import { startUpdateWatch } from './lib/updateWatch';

  onMount(() => {
    void names.load();
    // キーボードを出したときも、アプリの高さを見えている高さに合わせる。拡大（ピンチ・Ctrl+ホイールなど）は止める
    const stopViewport = startViewportSync();
    const stopZoom = startZoomGuard();
    // 右クリックの WebView の既定のメニュー（更新・保存・印刷など）は出さない
    const stopMenu = startContextMenuGuard();
    // アプリのアップデートを裏で確認し、新しい版があれば知らせる
    const stopUpdate = startUpdateWatch();
    return () => {
      stopViewport();
      stopZoom();
      stopMenu();
      stopUpdate();
    };
  });

  // 外部リンクは、OS の既定のブラウザーで開く
  onMount(() => {
    const onClick = (e: MouseEvent) => {
      const href = externalLinkHref(e, location.origin);
      if (!href) return;
      e.preventDefault();
      void import('@tauri-apps/plugin-opener').then((m) => m.openUrl(href));
    };
    document.addEventListener('click', onClick);
    return () => document.removeEventListener('click', onClick);
  });

  // 配布版でも Ctrl+Shift+I（F12）で開発者ツールを開く（デスクトップのみ）
  onMount(() => {
    if (!isTauri() || isAndroid()) return;
    const onKey = (e: KeyboardEvent) => {
      const ctrlShiftI = e.ctrlKey && e.shiftKey && (e.key === 'I' || e.key === 'i');
      if (!ctrlShiftI && e.key !== 'F12') return;
      e.preventDefault();
      void import('@tauri-apps/api/core').then(({ invoke }) => invoke('open_devtools'));
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  // PC の Alt+←: Android の戻るボタンと同じく、サブ画面を 1 つ閉じる
  onMount(() => {
    if (isAndroid()) return;
    const onKey = (e: KeyboardEvent) => {
      if (!isBackKey(e) || !ui.back()) return;
      e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  // Android の戻るボタンで、開いているサブ画面を 1 つずつ閉じる（閉じるものが無ければ、アプリが裏に回る）
  $effect(() => {
    if (!isAndroid()) return;
    const nav = startBackNav({ depth: () => ui.depth(), back: () => ui.back() }, history, window, { depthKey: 'offsaveDepth' });
    $effect(() => {
      void ui.depth();
      untrack(() => nav.sync());
    });
    return () => nav.stop();
  });

  // 「アプリ情報」を開いたときは、友達には知らせず、アプリ内の「Android の設定」に案内される（無効化・アンインストールのボタンがある画面だけを避けて、項目へ直接飛べる）
  $effect(() => {
    if (!isAndroid() || !session.saved) return;
    const check = () => {
      if (document.visibilityState !== 'visible') return;
      void native.takeSettingsRequest().then((requested) => requested && ui.openSettings('android')).catch(() => {});
    };
    check();
    document.addEventListener('visibilitychange', check);
    return () => document.removeEventListener('visibilitychange', check);
  });

  // 通知（友達のイベント）は、画面を開いていなくても取り直す（ナビゲーションの点のため）
  $effect(() => {
    if (!session.saved) return;
    events.start();
    protection.start();
    return () => {
      events.stop();
      protection.stop();
    };
  });

  const items = $derived<NavBarItem[]>([
    { id: 'home', label: 'ホーム', icon: House },
    { id: 'rules', label: 'ルール', icon: ListChecks },
    { id: 'friends', label: '友達', icon: Users },
    { id: 'events', label: '通知', icon: Bell, badge: ui.tab === 'events' ? null : events.unseen },
    { id: 'settings', label: '設定', icon: Settings, alert: protection.incomplete ? '監視が不完全' : showUpdateBadge(updater.state) ? UPDATE_BADGE_LABEL : null },
  ]);
</script>

{#if !isTauri()}
  <p class="only-tauri">offsave は Tauri 版（Windows・Linux・Android）でだけ動きます。</p>
{:else if !session.saved}
  <Setup />
{:else}
  <div class="shell" class:shell-mobile={ui.isMobile}>
    <main class="shell-main">
      {#if ui.tab === 'home'}
        <Home />
      {:else if ui.tab === 'rules'}
        <Rules />
      {:else if ui.tab === 'friends'}
        <Friends />
      {:else if ui.tab === 'events'}
        <Events />
      {:else}
        <Settings_ />
      {/if}
    </main>
    <NavBar
      id="nav-bar"
      class="app-nav-bar {ui.isMobile ? '' : 'nav-bar-side'}"
      label="ナビゲーション"
      {items}
      selected={ui.tab}
      onselect={(id) => ui.selectTab(id as Tab)}
    />
  </div>
{/if}
<Toasts toasts={ui.shell.toasts} dismiss={(id) => ui.shell.dismiss(id)} />
<ConfirmDialog request={ui.shell.confirmReq} />

<style>
  .only-tauri {
    padding: var(--space-8);
  }
</style>
