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
    assertEquals("com.android.chrome:id/url_bar", Browsers.urlBarId("com.android.chrome"))
    assertEquals("org.mozilla.firefox:id/mozac_browser_toolbar_url_view", Browsers.urlBarId("org.mozilla.firefox"))
    assertEquals(false, Browsers.supported("com.example.browser"))
  }
}
