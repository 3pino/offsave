package dev.offsave.apkupdater

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class UrlRulesTest {
  @Test
  fun releaseAssetsOfThisRepositoryAreAllowed() {
    assertTrue(UrlRules.isAllowed("https://github.com/3pino/offsave/releases/download/v0.2.0/offsave_0.2.0_android-arm64.apk"))
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
