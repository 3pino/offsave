package dev.offsave.enforcer

/**
 * 対応しているブラウザと、URL バー（アドレスバー）から見ているサイトを取り出す処理。
 *
 * 取るのはホスト名だけ（`example.com`）。パス・クエリ・フラグメントは、読んだ時点で捨てて、保存も送信もしない
 * （トークンなどの秘密が URL に載ることがあるため）。
 */
object Browsers {
  /** パッケージ → URL バーの view ID（リソース名の `id/` より後ろ）。 */
  private val URL_BAR_IDS: Map<String, String> = mapOf(
    // 検証済み: Chrome（エミュレーター）
    "com.android.chrome" to "url_bar",
    // 次のものは、同じ系統の実装（Chromium・Gecko）なので同じ ID の見込みだが、実機での確認が必要
    "com.chrome.beta" to "url_bar",
    "com.chrome.dev" to "url_bar",
    "com.chrome.canary" to "url_bar",
    "com.brave.browser" to "url_bar",
    "com.microsoft.emmx" to "url_bar",
    "com.vivaldi.browser" to "url_bar",
    "org.mozilla.firefox" to "mozac_browser_toolbar_url_view",
    "org.mozilla.firefox_beta" to "mozac_browser_toolbar_url_view",
    "org.mozilla.fenix" to "mozac_browser_toolbar_url_view",
    "com.sec.android.app.sbrowser" to "location_bar_edit_text",
  )

  fun supported(pkg: String): Boolean = URL_BAR_IDS.containsKey(pkg)

  /** URL バーの view ID（`<package>:id/<name>`）。 */
  fun urlBarId(pkg: String): String? = URL_BAR_IDS[pkg]?.let { "$pkg:id/$it" }

  private val HOST = Regex("^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$")

  /**
   * URL バーの文字からホスト名を取り出す。サイトでなければ（検索の案内文・新しいタブ・入力途中など）null。
   * `www.` は外す。
   */
  fun hostOf(text: String): String? {
    var s = text.trim().lowercase()
    if (s.isEmpty() || s.any { it.isWhitespace() }) return null
    s = s.substringAfter("://", s)
    if (s.startsWith("chrome:") || s.startsWith("about:") || s.startsWith("file:")) return null
    // パス・クエリ・フラグメントを捨てる
    s = s.substringBefore('/').substringBefore('?').substringBefore('#')
    // ユーザー情報とポート
    s = s.substringAfterLast('@').substringBefore(':')
    s = s.removePrefix("www.")
    return s.takeIf { HOST.matches(it) }
  }
}
