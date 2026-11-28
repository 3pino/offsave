package dev.offsave.enforcer

import android.app.AppOpsManager
import android.app.usage.UsageEvents
import android.app.usage.UsageStatsManager
import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.os.Process
import android.os.SystemClock
import android.provider.Settings
import android.util.Log

/**
 * 「enabled_accessibility_services」（コロン区切りの `パッケージ/クラス`）の読み書き。
 * 外すのは offsave のサービスだけで、ほかのアプリの項目には触れない。
 */
object ServiceList {
  /** 項目が、このパッケージ・クラスのサービスか。クラスは `.Name`（短い形）のこともある。 */
  fun isSame(entry: String, pkg: String, cls: String): Boolean {
    val (p, c) = entry.split('/', limit = 2).let { if (it.size == 2) it[0] to it[1] else return false }
    val full = if (c.startsWith(".")) p + c else c
    return p == pkg && full == cls
  }

  /** 自分を除いた値。残りが無ければ null（空文字ではなく、設定ごと消す）。 */
  fun without(value: String?, pkg: String, cls: String): String? =
    value.orEmpty().split(':').filter { it.isNotBlank() && !isSame(it, pkg, cls) }
      .joinToString(":").ifEmpty { null }

  /** 自分を加えた値（すでにあれば、そのまま）。 */
  fun with(value: String?, pkg: String, cls: String): String {
    val others = value.orEmpty().split(':').filter { it.isNotBlank() }
    return if (others.any { isSame(it, pkg, cls) }) others.joinToString(":") else (others + "$pkg/$cls").joinToString(":")
  }

  fun contains(value: String?, pkg: String, cls: String): Boolean =
    value.orEmpty().split(':').any { isSame(it, pkg, cls) }
}

/** 一時停止をいつ終えるかの判定（画面や時計に依存しない部分）。 */
object PausePlan {
  /** これを超えたら、アプリがまだ前面でも戻す（保護が戻らないままにならないように）。 */
  const val MAX_MS = 30 * 60_000L

  /** 前面が分からない（アプリの切り替えの途中など）まま、待つ長さ。 */
  const val UNKNOWN_GRACE_MS = 3_000L

  sealed class Step {
    /** そのまま続ける。`pkg` は、いま前面の決済アプリ。 */
    data class Keep(val pkg: String) : Step()
    data object End : Step()
  }

  /**
   * @param fg いま前面のアプリ（分からなければ null）
   * @param pausePackages 一時停止の対象のパッケージ
   * @param ignored 前面と見なさないもの（権限のダイアログなど）。無視して、いまのまま続ける
   * @param unknownMs fg が分からない状態が続いている長さ
   */
  fun next(
    fg: String?,
    current: String,
    pausePackages: Set<String>,
    ignored: Set<String>,
    screenOn: Boolean,
    elapsedMs: Long,
    unknownMs: Long,
  ): Step = when {
    !screenOn || elapsedMs > MAX_MS -> Step.End
    fg == null -> if (unknownMs > UNKNOWN_GRACE_MS) Step.End else Step.Keep(current)
    fg in ignored -> Step.Keep(current)
    fg in pausePackages -> Step.Keep(fg)
    else -> Step.End
  }
}

/**
 * 決済アプリなど、Accessibility を使っているデバイスでは動かないアプリのための、一時停止。
 *
 * 友達が決めたアプリが前面に来たら、offsave の Accessibility だけを自分で外し、そのアプリから離れたら戻す。
 * 外している間は、使用状況（UsageStats）で前面のアプリを見て、利用時間は数え続ける（制限の表示だけができない）。
 * 外す・戻すは友達に知らせる。戻らないままにならないよう、画面オフ・時間切れ・再起動でも戻す。
 *
 * 設定の書き込み（WRITE_SECURE_SETTINGS、adb で 1 回付ける）と、使用状況へのアクセスの、両方が要る。
 */
class Pause(private val agent: Agent) {
  companion object {
    private const val TAG = "offsave-pause"
    private const val ENABLED = "enabled_accessibility_services"
    private const val ENABLED_FLAG = "accessibility_enabled"
    private const val TICK_MS = 700L

    /** 前面と見なさないもの。権限のダイアログなどが前面に出ても、決済の最中の一時停止を終えない。 */
    private val IGNORED = setOf(
      "com.android.systemui",
      "android",
      "com.google.android.permissioncontroller",
      "com.android.permissioncontroller",
    )
  }

  private val context: Context = agent.context
  private val main = Handler(Looper.getMainLooper())
  private val serviceClass = GuardService::class.java.name

  private var fg: String? = null
  private var unknownSince = 0L
  private var lastQuery = 0L

  /** 戻した直後、サービスがつながり直すまでの間は、保護が外れたとは見せない。 */
  @Volatile private var restoringUntil = 0L

  /** 友達に送る「一時停止中」。外している間と、戻した直後のつなぎ直しの間。 */
  val pausedFlag: Boolean
    get() = activePackage != null || (agent.guard == null && SystemClock.elapsedRealtime() < restoringUntil)

  val canWriteSecure: Boolean
    get() = context.checkSelfPermission("android.permission.WRITE_SECURE_SETTINGS") == PackageManager.PERMISSION_GRANTED

  val hasUsageAccess: Boolean
    get() {
      val ops = context.getSystemService(Context.APP_OPS_SERVICE) as AppOpsManager
      val mode = if (Build.VERSION.SDK_INT >= 29) {
        ops.unsafeCheckOpNoThrow(AppOpsManager.OPSTR_GET_USAGE_STATS, Process.myUid(), context.packageName)
      } else {
        @Suppress("DEPRECATION")
        ops.checkOpNoThrow(AppOpsManager.OPSTR_GET_USAGE_STATS, Process.myUid(), context.packageName)
      }
      return mode == AppOpsManager.MODE_ALLOWED
    }

  val ready: Boolean get() = canWriteSecure && hasUsageAccess

  /** 一時停止中の決済アプリ（していなければ null）。再起動しても残る。 */
  val activePackage: String? get() = agent.prefs.pausePackage

  private fun pausePackages(): Set<String> =
    agent.tracker.policy?.pauseApps.orEmpty().filter { it.startsWith("android:") }.map { it.removePrefix("android:") }.toSet()

  /** 前面のアプリが変わった。対象のアプリなら、Accessibility を外す。 */
  fun onForeground(pkg: String?) {
    if (pkg == null || activePackage != null) return
    if (pkg !in pausePackages() || !ready) return
    begin(pkg)
  }

  private fun begin(pkg: String) {
    val prefs = agent.prefs
    prefs.pausePackage = pkg
    prefs.pauseStartedAt = System.currentTimeMillis()
    if (!write { ServiceList.without(it, context.packageName, serviceClass) }) {
      prefs.pausePackage = null
      return
    }
    // 一時停止・再開は、友達に通知しない（対象のアプリは友達が決めていて、ハートビートの paused で状態は見える）
    startLoop(pkg, prefs.pauseStartedAt)
  }

  /** 一時停止の途中でプロセスが再開したとき（再起動・サービスの再開）に、続きを引き受ける。 */
  fun resume() {
    val pkg = activePackage ?: return
    if (!ready) {
      end(pkg, canRestore = canWriteSecure)
      return
    }
    startLoop(pkg, agent.prefs.pauseStartedAt)
  }

  /** 画面が消えた。 */
  fun onScreenOff() = restoreNow()

  /** 一時停止中なら、いま戻す（画面オフ・連携の解除）。 */
  fun restoreNow() {
    val pkg = activePackage ?: return
    main.post { end(pkg, canRestore = canWriteSecure) }
  }

  private fun startLoop(pkg: String, since: Long) {
    main.removeCallbacks(tick)
    fg = pkg
    unknownSince = 0
    lastQuery = since
    main.post { tickNow() }
  }

  private val tick = Runnable { tickNow() }

  private fun tickNow() {
    val current = activePackage ?: return
    val now = System.currentTimeMillis()
    readForeground(now)
    val power = context.getSystemService(Context.POWER_SERVICE) as PowerManager
    val imes = agent.imes()
    val step = PausePlan.next(
      fg = fg,
      current = current,
      pausePackages = pausePackages(),
      ignored = IGNORED + imes,
      screenOn = power.isInteractive,
      elapsedMs = now - agent.prefs.pauseStartedAt,
      unknownMs = if (fg == null && unknownSince > 0) now - unknownSince else 0,
    )
    when (step) {
      is PausePlan.Step.End -> end(current, canRestore = canWriteSecure)
      is PausePlan.Step.Keep -> {
        if (step.pkg != current) {
          agent.prefs.pausePackage = step.pkg
        }
        main.postDelayed(tick, TICK_MS)
      }
    }
  }

  /** UsageStats の出来事から、前面のアプリを更新する。変わったら、利用時間の記録にも反映する。 */
  private fun readForeground(now: Long) {
    val usm = context.getSystemService(Context.USAGE_STATS_SERVICE) as UsageStatsManager
    val before = fg
    try {
      // 少し重ねて読む（出来事の記録が遅れて届くことがある）。ただし、始めた時刻より前は読まない（前のアプリの出来事を、いまの前面と取り違えない）
      val startedAt = agent.prefs.pauseStartedAt
      val events = usm.queryEvents(maxOf(startedAt, lastQuery - 500), now)
      val e = UsageEvents.Event()
      while (events.hasNextEvent()) {
        events.getNextEvent(e)
        if (e.timeStamp < startedAt) continue
        when (e.eventType) {
          @Suppress("DEPRECATION") UsageEvents.Event.MOVE_TO_FOREGROUND -> fg = e.packageName
          @Suppress("DEPRECATION") UsageEvents.Event.MOVE_TO_BACKGROUND -> if (fg == e.packageName) fg = null
        }
      }
      lastQuery = now
    } catch (e: Exception) {
      Log.w(TAG, "前面を読めません", e)
    }
    if (fg == null && unknownSince == 0L) unknownSince = now
    if (fg != null) unknownSince = 0
    if (fg != before && fg != null && fg !in IGNORED) agent.onForeground(fg)
  }

  /** 戻す。`canRestore` が false（権限が無い）なら、戻せないので、状態だけ消す（友達には「保護が外れた」として届く）。 */
  private fun end(pkg: String, canRestore: Boolean) {
    main.removeCallbacks(tick)
    val prefs = agent.prefs
    if (prefs.pausePackage == null) return
    if (canRestore) {
      val ok = write { ServiceList.with(it, context.packageName, serviceClass) }
      if (ok) {
        try {
          Settings.Secure.putInt(context.contentResolver, ENABLED_FLAG, 1)
        } catch (e: Exception) {
          Log.w(TAG, "accessibility_enabled を戻せません", e)
        }
      } else {
        // 戻せなかった。続けて試す（状態は残す）
        main.postDelayed(tick, 5_000L)
        return
      }
    }
    if (canRestore) restoringUntil = SystemClock.elapsedRealtime() + 10_000L
    prefs.pausePackage = null
    // 戻せなかったとき（権限が無い）だけは、保護が外れたままなので、友達に知らせる
    if (!canRestore) {
      agent.link.report("protection_paused", "保護を戻せませんでした（設定の書き込みの権限がありません）")
    }
    // サービスがつながり直したところで、ハートビートを送る（Agent.attachGuard）
  }

  /** 設定を書く。`change` に、いまの値を渡し、新しい値（null なら消す）を返す。 */
  private fun write(change: (String?) -> String?): Boolean = try {
    val cr = context.contentResolver
    val current = Settings.Secure.getString(cr, ENABLED)
    val next = change(current)
    // 空にするときは、空文字ではなく、設定ごと消す（空文字だと、サービスが残ることがある）
    Settings.Secure.putString(cr, ENABLED, next)
    true
  } catch (e: SecurityException) {
    Log.w(TAG, "設定を書けません", e)
    false
  } catch (e: Exception) {
    Log.w(TAG, "設定を書けません", e)
    false
  }
}
