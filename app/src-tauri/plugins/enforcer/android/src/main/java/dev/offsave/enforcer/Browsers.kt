package dev.offsave.enforcer

/**
 * 対応しているブラウザと、URL バー（アドレスバー）から見ているサイトを取り出す処理。
 *
 * 取るのはホスト名だけ（`example.com`）。パス・クエリ・フラグメントは、読んだ時点で捨てて、保存も送信もしない
 * （トークンなどの秘密が URL に載ることがあるため）。
 */
object Browsers {
  /**
   * Compose のツールバーの場所。Compose の中のノードは view ID では見つからないので、外側の View（`container`）を ID で見つけて、その中をたどって
   * testTag（`tag`。パッケージ名なしの ID）のノードを探す。文字は、あれば `text`、空なら `contentDescription` から読む。
   */
  class ComposeBar(val containerId: String, val tag: String)

  private val GECKO_VIEW = "mozac_browser_toolbar_url_view" // 古い View 版のツールバー
  private val GECKO_COMPOSE = "composable_toolbar" to "ADDRESSBAR_URL_BOX" // Firefox 161 のツールバー
  private val GECKO_BARS = listOf(GECKO_VIEW)
  private val URL_BAR_IDS: Map<String, List<String>> = mapOf(
    // 検証済み: Chrome（エミュレーター）
    "com.android.chrome" to listOf("url_bar"),
    // 同じ系統の実装（Chromium）なので同じ ID の見込みだが、実機での確認が必要
    "com.chrome.beta" to listOf("url_bar"),
    "com.chrome.dev" to listOf("url_bar"),
    "com.chrome.canary" to listOf("url_bar"),
    "com.brave.browser" to listOf("url_bar"),
    "com.microsoft.emmx" to listOf("url_bar"),
    "com.vivaldi.browser" to listOf("url_bar"),
    "com.sec.android.app.sbrowser" to listOf("location_bar_edit_text"),
    // Firefox（Fenix）。検証済み: Firefox 161 の新しいツールバー（Compose）。古い View 版の ID も候補に入れてある
    "org.mozilla.firefox" to GECKO_BARS,
    "org.mozilla.firefox_beta" to GECKO_BARS,
    "org.mozilla.fenix" to GECKO_BARS,
    "org.mozilla.fennec_aurora" to GECKO_BARS,
    // Fenix をもとにした派生（同じ見込み。未確認）
    "org.mozilla.fennec_fdroid" to GECKO_BARS,
    "io.github.forkmaintainers.iceraven" to GECKO_BARS,
    "us.spotco.fennec_dos" to GECKO_BARS,
  )

  fun supported(pkg: String): Boolean = URL_BAR_IDS.containsKey(pkg)

  /** URL バーの view ID の候補（`findAccessibilityNodeInfosByViewId` に渡す形）。 */
  fun urlBarIds(pkg: String): List<String> = URL_BAR_IDS[pkg].orEmpty().map { "$pkg:id/$it" }

  /** Compose のツールバー（あれば）。 */
  fun composeBar(pkg: String): ComposeBar? =
    if (URL_BAR_IDS[pkg] === GECKO_BARS) ComposeBar("$pkg:id/${GECKO_COMPOSE.first}", GECKO_COMPOSE.second) else null

  /** Compose のツールバーは、URL を `text` でなく説明文（`" example.com. Search or enter address"`）で返す。先頭の URL だけを取り出す。 */
  fun hostOfDescription(desc: String): String? {
    val lead = desc.trim().takeWhile { it.code in 0x21..0x7e }.trimEnd('.')
    return hostOf(lead)
  }

  /** URL バーのノードの文字からホスト名を取る（`text` を優先し、空なら説明文）。 */
  fun hostOfBar(text: String?, desc: String?): String? =
    if (!text.isNullOrBlank()) hostOf(text) else desc?.let { hostOfDescription(it) }

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
