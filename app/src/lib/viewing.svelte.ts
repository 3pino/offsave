import { session } from './session.svelte';

/**
 * ホーム・ルールで見ている人（自分、または承認済みの友達）。見出しの名前を押して切り替える。
 * 2 つの画面で同じ人を見る。保存しない（アプリを開き直すと自分に戻る）。
 */
class Viewing {
  private who = $state.raw<{ id: string; name: string } | null>(null);

  /** 見ている人の id */
  get id(): string {
    return this.who?.id ?? session.saved?.userId ?? '';
  }
  /** 自分を見ているか */
  get mine(): boolean {
    return this.who === null || this.who.id === session.saved?.userId;
  }
  /** 見ている人の呼び方（「alice さん」） */
  get name(): string {
    return `${this.who?.name ?? session.saved?.userName ?? ''} さん`;
  }

  /** `null` なら自分 */
  set(who: { id: string; name: string } | null) {
    this.who = who;
  }

  /** この友達を見ていたら、自分に戻す（友達を解消したとき） */
  drop(userId: string) {
    if (this.who?.id === userId) this.who = null;
  }
}

export const viewing = new Viewing();
