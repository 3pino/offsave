package dev.offsave.enforcer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TrackerTest {
  private val t0 = 1_795_575_600_000L // 2026-11-25 12:00 JST

  private fun policyJson(
    seconds: Long = 120,
    others: String = "[]",
    unlockUntil: Long = 0,
    serverTime: Long = t0,
  ) = """
    {"server_time": $serverTime, "clock": {"utc_offset_min": 540, "day_boundary_min": 240},
     "rules": [{"id": "r", "name": "ゲーム", "targets": [{"label": "g", "keys": ["android:g"]}],
                "limit": {"type": "daily_total", "seconds": $seconds}, "enabled": true}],
     "overrides": [], "unlock_until": $unlockUntil, "last_unlock_end": 0, "unlock_minutes": 15,
     "others": $others}
  """.trimIndent()

  @Test
  fun sessionsBecomeSegmentsAndPendingUntilAcked() {
    val tr = Tracker(MemoryStore())
    tr.onForeground("android:g", t0)
    tr.onForeground("android:other", t0 + 30_000)
    tr.cut(t0 + 50_000)
    val sent = tr.beginUploadSegments()
    assertEquals(
      listOf(Segment("android:g", t0, t0 + 30_000), Segment("android:other", t0 + 30_000, t0 + 50_000)),
      sent,
    )
    // 返事が来る前は消えない（再送できる）
    assertEquals(2, tr.beginUploadSegments().size)
    tr.applyPolicy(policyJson())
    assertTrue(tr.beginUploadSegments().isEmpty())
  }

  @Test
  fun usageOnOtherDevicesCountsTowardTheLimit() {
    val tr = Tracker(MemoryStore())
    val others = """[{"key": "android:g", "start": ${t0 - 100_000}, "end": ${t0 - 10_000}}]"""
    tr.applyPolicy(policyJson(seconds = 120, others = others))
    tr.onForeground("android:g", t0)
    // 他のデバイス 90 秒 + 今 20 秒 = 110 秒 < 120 秒
    assertEquals(Verdict.Allow, tr.decide(t0 + 20_000).verdict)
    // 130 秒 → 猶予 → 猶予のあとにブロック
    val d = tr.decide(t0 + 40_000)
    assertTrue(d.verdict is Verdict.Grace)
    assertTrue(tr.decide(t0 + 110_000).verdict is Verdict.Block)
  }

  @Test
  fun graceSurvivesRestart() {
    val store = MemoryStore()
    val tr = Tracker(store)
    tr.applyPolicy(policyJson(seconds = 0))
    tr.onForeground("android:g", t0)
    assertTrue(tr.decide(t0).verdict is Verdict.Grace)
    // 再起動（猶予の開始が保存されていて、アプリを開き直しても戻らない）
    val tr2 = Tracker(store)
    tr2.onForeground("android:g", t0 + 120_000)
    assertTrue(tr2.decide(t0 + 120_000).verdict is Verdict.Block)
  }

  @Test
  fun unlockAllowsAndIsQueuedForTheServer() {
    val tr = Tracker(MemoryStore())
    tr.applyPolicy(policyJson(seconds = 0))
    tr.onForeground("android:g", t0)
    val until = tr.startUnlock("調べ物のため", t0)
    assertEquals(t0 + 15 * 60_000, until)
    assertEquals(Verdict.Allow, tr.decide(t0 + 1000).verdict)
    val unlocks = tr.beginUploadUnlocks()
    assertEquals(listOf(PendingUnlock("調べ物のため", t0, until)), unlocks)
    // 解除が終わると、新しいイベントとして猶予がもう一度つく
    assertTrue(tr.decide(until + 1).verdict is Verdict.Grace)
    tr.applyPolicy(policyJson(seconds = 0, serverTime = until + 2))
    assertTrue(tr.beginUploadUnlocks().isEmpty())
  }

  private fun domainPolicy(seconds: Long) = """
    {"server_time": $t0, "clock": {"utc_offset_min": 540, "day_boundary_min": 240},
     "rules": [{"id": "d", "name": "動画", "targets": [{"label": "yt", "keys": ["domain:example.com"]}],
                "limit": {"type": "daily_total", "seconds": $seconds}, "enabled": true}],
     "overrides": [], "unlock_until": 0, "last_unlock_end": 0, "unlock_minutes": 15, "others": []}
  """.trimIndent()

  @Test
  fun siteInBrowserIsTrackedAndLimitedSeparatelyFromTheApp() {
    val tr = Tracker(MemoryStore())
    tr.applyPolicy(domainPolicy(seconds = 30))
    tr.onForeground("android:browser", t0)
    assertEquals(Verdict.Allow, tr.decide(t0 + 1000).verdict)
    tr.onDomain("example.com", t0 + 1000)
    assertEquals("domain:example.com", tr.foregroundKey)
    assertEquals(Verdict.Allow, tr.decide(t0 + 20_000).verdict)
    val d = tr.decide(t0 + 40_000)
    assertTrue(d.verdict is Verdict.Grace)
    // 猺予の対象はサイト（友達への通知で「どのサイトか」を伝える）
    assertEquals(listOf("domain:example.com"), tr.graceSubjects.values.toList())
    // サイトを離れれば、区間が記録され、判定は Allow に戻る
    tr.onDomain(null, t0 + 50_000)
    assertEquals(Verdict.Allow, tr.decide(t0 + 51_000).verdict)
    assertTrue(tr.beginUploadSegments().any { it.key == "domain:example.com" })
  }

  @Test
  fun leavingTheBrowserClosesTheSiteToo() {
    val tr = Tracker(MemoryStore())
    tr.applyPolicy(domainPolicy(seconds = 600))
    tr.onForeground("android:browser", t0)
    tr.onDomain("example.com", t0)
    tr.onForeground("android:other", t0 + 10_000)
    assertEquals("android:other", tr.foregroundKey)
    val keys = tr.beginUploadSegments().map { it.key }
    assertTrue(keys.contains("domain:example.com") && keys.contains("android:browser"))
  }

  @Test
  fun screenOffStopsCounting() {
    val tr = Tracker(MemoryStore())
    tr.applyPolicy(policyJson(seconds = 100))
    tr.onForeground("android:g", t0)
    tr.onForeground(null, t0 + 50_000)
    assertEquals(Verdict.Allow, tr.decide(t0 + 1_000_000).verdict)
    assertEquals(null, tr.currentKey)
  }
}
