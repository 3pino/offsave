package dev.offsave.enforcer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SettingsGuardTest {
  @Test
  fun appInfoWithDangerousButtonsIsRecognized() {
    val page = listOf("アプリ情報", "offsave", "開く", "アンインストール", "強制停止")
    assertTrue(GuardService.looksLikeOurAppInfo(page, "offsave"))
    assertTrue(GuardService.looksLikeOurAppInfo(listOf("App info", "offsave", "Open", "Uninstall", "Force stop"), "offsave"))
  }

  @Test
  fun otherPagesOfOurAppAreNotRedirected() {
    // 通知の設定: 通知チャンネルの名前に「停止」を含むが、ボタンではない
    val notifications = listOf("offsave", "offsave のすべての通知", "保護の一時停止・再開", "保護の無効化・不審な操作", "制限に達した")
    assertFalse(GuardService.looksLikeOurAppInfo(notifications, "offsave"))
    assertFalse(GuardService.looksLikeOurAppInfo(listOf("offsave", "権限", "通知", "許可しない"), "offsave"))
    assertFalse(GuardService.looksLikeOurAppInfo(listOf("offsave", "バックグラウンドのデータ"), "offsave"))
  }

  @Test
  fun pagesOfOtherAppsAreIgnored() {
    assertFalse(GuardService.looksLikeOurAppInfo(listOf("メモ", "アンインストール", "強制停止"), "offsave"))
  }

  @Test
  fun onlyAnOnToOffChangeIsReported() {
    val on = mapOf("notifications" to true, "battery" to true, "background_data" to true, "usage_access" to false)
    // 変わらない・off から on・初めから off は、通知しない
    assertEquals(emptyList<String>(), SettingsWatch.lost(on, on))
    assertEquals(emptyList<String>(), SettingsWatch.lost(on, on + ("usage_access" to true)))
    assertEquals(emptyList<String>(), SettingsWatch.lost(emptyMap(), on))
    // on だったものが off になった
    assertEquals(listOf("notifications", "battery"), SettingsWatch.lost(on, on + ("notifications" to false) + ("battery" to false)))
  }

  @Test
  fun watchedStateRoundTrips() {
    val s = mapOf("notifications" to true, "battery" to false)
    assertEquals(s, SettingsWatch.decode(SettingsWatch.encode(s)))
    assertEquals(emptyMap<String, Boolean>(), SettingsWatch.decode(null))
    assertEquals(emptyMap<String, Boolean>(), SettingsWatch.decode("壊れた"))
  }
}
