package dev.offsave.apkupdater

/** ダウンロードしてよい URL の規則（JVM で試験できるよう、Android の API を使わない）。 */
object UrlRules {
  /** この版の APK は、offsave のリリースの資産だけ。ほかの場所のファイルは取らない。 */
  const val ALLOWED_PREFIX = "https://github.com/3pino/offsave/releases/download/"

  fun isAllowed(url: String): Boolean =
    url.startsWith(ALLOWED_PREFIX) &&
      url.endsWith(".apk") &&
      // `..` や `@` で、許可の接頭辞から外れた場所を指させない
      !url.contains("..") && !url.contains('@') && !url.contains('\\') && url.none { it.isWhitespace() }
}
