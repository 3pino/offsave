package dev.offsave.enforcer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BrowsersTest {
  @Test
  fun hostIsExtractedAndEverythingElseIsDropped() {
    assertEquals("example.com", Browsers.hostOf("example.com"))
    assertEquals("example.com", Browsers.hostOf("https://www.example.com/a/b?token=secret#frag"))
    assertEquals("sub.example.co.jp", Browsers.hostOf("sub.example.co.jp:8080/x"))
    assertEquals("evil.com", Browsers.hostOf("user:pw@evil.com/x"))
    assertEquals("example.com", Browsers.hostOf("  EXAMPLE.com  "))
  }

  @Test
  fun nonSitesAreIgnored() {
    assertNull(Browsers.hostOf(""))
    assertNull(Browsers.hostOf("Search or type URL"))
    assertNull(Browsers.hostOf("chrome://newtab"))
    assertNull(Browsers.hostOf("localhost:3000"))
    assertNull(Browsers.hostOf("you"))
    assertNull(Browsers.hostOf("foo bar.com"))
  }

  @Test
  fun urlBarIds() {
    assertTrue(Browsers.supported("com.android.chrome"))
    assertEquals(listOf("com.android.chrome:id/url_bar"), Browsers.urlBarIds("com.android.chrome"))
    assertEquals(listOf("org.mozilla.firefox:id/mozac_browser_toolbar_url_view"), Browsers.urlBarIds("org.mozilla.firefox"))
    val compose = Browsers.composeBar("org.mozilla.firefox")!!
    assertEquals("org.mozilla.firefox:id/composable_toolbar", compose.containerId)
    assertEquals("ADDRESSBAR_URL_BOX", compose.tag)
    assertNull(Browsers.composeBar("com.android.chrome"))
    assertEquals(false, Browsers.supported("com.example.browser"))
    assertEquals(emptyList<String>(), Browsers.urlBarIds("com.example.browser"))
  }

  @Test
  fun firefoxComposeToolbarReadsTheDescription() {
    // Firefox 161 で実際に返ってきた文字
    assertEquals("example.com", Browsers.hostOfBar("", " example.com. Search or enter address"))
    assertEquals("example.com", Browsers.hostOfBar(null, "example.com。検索またはアドレスを入力"))
    assertEquals("example.com", Browsers.hostOfBar(null, "https://www.example.com/a?q=1. Search or enter address"))
    // 新しいタブ・検索語はサイトではない
    assertNull(Browsers.hostOfBar("", " Search or enter address"))
    assertNull(Browsers.hostOfBar(null, null))
    // text があるときは text を使う（View 版・Chrome）
    assertEquals("example.com", Browsers.hostOfBar("example.com", "いろいろ"))
  }
}
