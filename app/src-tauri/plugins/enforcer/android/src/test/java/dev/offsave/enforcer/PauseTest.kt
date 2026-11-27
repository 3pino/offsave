package dev.offsave.enforcer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PauseTest {
  private val pkg = "dev.offsave.app"
  private val cls = "dev.offsave.enforcer.GuardService"

  @Test
  fun removesOnlyOurService() {
    val v = "com.bitwarden/com.bitwarden.A11y:dev.offsave.app/dev.offsave.enforcer.GuardService:net.dinglisch.android.taskerm/.Svc"
    assertEquals("com.bitwarden/com.bitwarden.A11y:net.dinglisch.android.taskerm/.Svc", ServiceList.without(v, pkg, cls))
  }

  @Test
  fun removingTheLastOneDeletesTheSettingInsteadOfWritingAnEmptyString() {
    assertNull(ServiceList.without("dev.offsave.app/dev.offsave.enforcer.GuardService", pkg, cls))
    assertNull(ServiceList.without(null, pkg, cls))
    assertNull(ServiceList.without("", pkg, cls))
  }

  @Test
  fun understandsTheShortClassName() {
    // 端末によっては `パッケージ/.クラス` の形で入っている
    assertTrue(ServiceList.isSame("dev.offsave.app/.Svc", "dev.offsave.app", "dev.offsave.app.Svc"))
    assertFalse(ServiceList.isSame("other.app/dev.offsave.enforcer.GuardService", pkg, cls))
    assertFalse(ServiceList.isSame("garbage", pkg, cls))
  }

  @Test
  fun addingBackKeepsTheOthersAndDoesNotDuplicate() {
    assertEquals(
      "com.bitwarden/com.bitwarden.A11y:dev.offsave.app/dev.offsave.enforcer.GuardService",
      ServiceList.with("com.bitwarden/com.bitwarden.A11y", pkg, cls),
    )
    val already = "dev.offsave.app/dev.offsave.enforcer.GuardService"
    assertEquals(already, ServiceList.with(already, pkg, cls))
    assertEquals(already, ServiceList.with(null, pkg, cls))
  }

  @Test
  fun roundTripKeepsEveryoneElse() {
    val v = "a/b.C:dev.offsave.app/dev.offsave.enforcer.GuardService:d/e.F"
    val off = ServiceList.without(v, pkg, cls)
    assertFalse(ServiceList.contains(off, pkg, cls))
    assertTrue(ServiceList.contains(ServiceList.with(off, pkg, cls), pkg, cls))
    assertEquals(setOf("a/b.C", "d/e.F"), off!!.split(':').toSet())
  }

  // ---- 一時停止を終える判定 ----

  private val pay = setOf("jp.ne.paypay.android.app")
  private val ignored = setOf("com.android.systemui")

  private fun next(fg: String?, screenOn: Boolean = true, elapsed: Long = 10_000, unknown: Long = 0) =
    PausePlan.next(fg, "jp.ne.paypay.android.app", pay, ignored, screenOn, elapsed, unknown)

  @Test
  fun keepsWhileTheAppIsInFront() {
    assertEquals(PausePlan.Step.Keep("jp.ne.paypay.android.app"), next("jp.ne.paypay.android.app"))
  }

  @Test
  fun endsWhenAnotherAppComesToFront() {
    assertEquals(PausePlan.Step.End, next("com.android.launcher3"))
  }

  @Test
  fun endsWhenTheScreenGoesOffOrTheTimeLimitPasses() {
    assertEquals(PausePlan.Step.End, next("jp.ne.paypay.android.app", screenOn = false))
    assertEquals(PausePlan.Step.End, next("jp.ne.paypay.android.app", elapsed = PausePlan.MAX_MS + 1))
  }

  @Test
  fun waitsBrieflyWhenTheForegroundIsUnknownThenEnds() {
    assertEquals(PausePlan.Step.Keep("jp.ne.paypay.android.app"), next(null, unknown = 1_000))
    assertEquals(PausePlan.Step.End, next(null, unknown = PausePlan.UNKNOWN_GRACE_MS + 1))
  }

  @Test
  fun ignoresSystemDialogsInTheMiddleOfAPayment() {
    assertEquals(PausePlan.Step.Keep("jp.ne.paypay.android.app"), next("com.android.systemui"))
  }

  @Test
  fun followsToAnotherPauseAppWithoutRestoring() {
    val both = pay + "jp.co.rakuten.pay"
    assertEquals(
      PausePlan.Step.Keep("jp.co.rakuten.pay"),
      PausePlan.next("jp.co.rakuten.pay", "jp.ne.paypay.android.app", both, ignored, true, 1_000, 0),
    )
  }
}
