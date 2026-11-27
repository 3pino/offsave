/** 例外 → 画面に出す文章 */
export function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
