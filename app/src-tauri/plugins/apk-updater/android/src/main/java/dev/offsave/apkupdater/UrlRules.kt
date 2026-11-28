package dev.offsave.apkupdater

/** ダウンロードしてよい URL の規則（JVM で試験できるよう、Android の API を使わない）。 */
object UrlRules {
  /** この版の APK は、offsave のリリースの資産だけ。ほかの場所のファイルは取らない。 */
  const val ALLOWED_PREFIX = "https://github.com/3pino/offsave/releases/download/"

  /** 最新のリリースへの入口。GitHub の API ではなく通常のページなので、API の回数の上限（未認証は 1 時間に 60 回）にかからない。 */
  const val LATEST_URL = "https://github.com/3pino/offsave/releases/latest"

  private val TAG_LOCATION = Regex("^https://github\\.com/3pino/offsave/releases/tag/([A-Za-z0-9._-]{1,40})$")

  /** `LATEST_URL` の転送先（`.../releases/tag/v1.2.3`）から、タグ名（`v1.2.3`）を取り出す。想定と違えば null。 */
  fun tagFromLocation(location: String?): String? = location?.let { TAG_LOCATION.matchEntire(it)?.groupValues?.get(1) }

  fun isAllowed(url: String): Boolean =
    url.startsWith(ALLOWED_PREFIX) &&
      url.endsWith(".apk") &&
      // `..` や `@` で、許可の接頭辞から外れた場所を指させない
      !url.contains("..") && !url.contains('@') && !url.contains('\\') && url.none { it.isWhitespace() }
}
