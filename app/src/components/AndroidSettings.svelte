<script lang="ts">
  import Bell from '@lucide/svelte/icons/bell';
  import BatteryMedium from '@lucide/svelte/icons/battery-medium';
  import Link from '@lucide/svelte/icons/link';
  import Lock from '@lucide/svelte/icons/lock';
  import Signal from '@lucide/svelte/icons/signal';
  import type { Component } from 'svelte';
  import Section from '@3pino/ui-kit-web/Section.svelte';
  import SettingRow from '@3pino/ui-kit-web/SettingRow.svelte';
  import { errorText } from '../lib/errors';
  import { native } from '../lib/native';
  import type { AndroidSettingKind } from '../lib/types';
  import { ui } from '../lib/ui.svelte';
  import Page from './Page.svelte';

  // 「アプリ情報」の画面には、無効化・アンインストール・強制停止のボタンがあるので、開くと自動でここに戻される（友達には通知しない）。
  // ここからは、アプリ情報を経由せず、各項目に直接飛べる。
  let { onback }: { onback: () => void } = $props();

  interface Item {
    kind: AndroidSettingKind;
    name: string;
    description: string;
    icon: Component;
    /** 飛ぶ前に出す注意。off にされると困る項目だけ持つ */
    warn?: string;
  }

  const ITEMS: Item[] = [
    {
      kind: 'notifications',
      name: '通知',
      description: '通知の種類ごとの音・重要度・オンオフ',
      icon: Bell,
      warn: '通知の全体のスイッチは off にしないでください。種類ごとの調整はできます。',
    },
    {
      kind: 'permissions',
      name: '権限',
      description: 'このアプリに許可している権限',
      icon: Lock,
      warn: '権限は off にしないでください。見るだけにしてください。',
    },
    {
      kind: 'data',
      name: 'モバイルデータの使用量',
      description: 'バックグラウンドのデータ・データセーバー',
      icon: Signal,
      warn: 'バックグラウンドのデータは off にしないでください。見るだけにしてください。',
    },
    {
      kind: 'battery',
      name: '電池の最適化',
      description: 'このアプリを電池の最適化から除外しているか',
      icon: BatteryMedium,
      warn: '電池の最適化は「最適化しない」のままにしてください。見るだけにしてください。',
    },
    { kind: 'open_by_default', name: 'デフォルトで開く', description: 'リンクをこのアプリで開くか', icon: Link },
  ];

  async function open(item: Item) {
    if (item.warn) {
      const ok = await ui.confirm({
        title: `「${item.name}」を開きます`,
        body: `${item.warn}\n\noff にすると、友達に通知されます。`,
        okLabel: '開く',
      });
      if (!ok) return;
    }
    try {
      if (!(await native.openAndroidSetting(item.kind))) ui.toast('このデバイスでは開けません', 'error');
    } catch (e) {
      ui.toast(errorText(e), 'error');
    }
  }
</script>

<Page title="Android の設定" {onback}>
  <Section title="このアプリの設定">
    <p class="empty">アプリ情報の画面を開かずに、各項目へ直接移動します。</p>
    {#each ITEMS as item (item.kind)}
      <SettingRow name={item.name} description={item.description} icon={item.icon} onclick={() => open(item)} />
    {/each}
  </Section>
</Page>
