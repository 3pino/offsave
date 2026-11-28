package dev.offsave.apkupdater

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class UrlRulesTest {
  @Test
  fun releaseAssetsOfThisRepositoryAreAllowed() {
    assertTrue(UrlRules.isAllowed("https://github.com/3pino/offsave/releases/download/v0.2.0/offsave_0.2.0_android-arm64.apk"))
  }

  @Test
  fun tagIsReadFromTheLatestReleaseRedirect() {
    assertEquals("v0.3.5", UrlRules.tagFromLocation("https://github.com/3pino/offsave/releases/tag/v0.3.5"))
    assertNull(UrlRules.tagFromLocation(null))
    assertNull(UrlRules.tagFromLocation("https://github.com/3pino/other/releases/tag/v1"))
    assertNull(UrlRules.tagFromLocation("https://evil.example/3pino/offsave/releases/tag/v1"))
    assertNull(UrlRules.tagFromLocation("https://github.com/3pino/offsave/releases/tag/v1/../x"))
    assertNull(UrlRules.tagFromLocation("https://github.com/login?return_to=x"))
  }

  @Test
  fun everythingElseIsRejected() {
    assertFalse(UrlRules.isAllowed("http://github.com/3pino/offsave/releases/download/v1/a.apk"))
    assertFalse(UrlRules.isAllowed("https://github.com/3pino/other/releases/download/v1/a.apk"))
    assertFalse(UrlRules.isAllowed("https://evil.example/3pino/offsave/releases/download/v1/a.apk"))
    assertFalse(UrlRules.isAllowed("https://github.com/3pino/offsave/releases/download/v1/a.exe"))
    assertFalse(UrlRules.isAllowed("https://github.com/3pino/offsave/releases/download/../../x/y/a.apk"))
    assertFalse(UrlRules.isAllowed("https://github.com/3pino/offsave/releases/download/@evil.example/a.apk"))
    assertFalse(UrlRules.isAllowed("https://github.com/3pino/offsave/releases/download/v1/a b.apk"))
  }
}
