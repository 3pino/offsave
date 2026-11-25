package dev.offsave.enforcer

import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.view.inputmethod.InputMethodManager
import org.json.JSONObject

/**
 * プロセスに 1 つの本体。前面のアプリの追跡（[Tracker]）、サーバーとの通信（[Link]）、
 * 制限の表示（[Overlay]）をつなぐ。Accessibility Service とフォアグラウンドサービスは、どちらもここを使う。
 */
class Agent private constructor(context: Context) {
  companion object {
    @Volatile private var instance: Agent? = null

    fun get(context: Context): Agent =
      instance ?: synchronized(this) {
        instance ?: Agent(context.applicationContext).also { instance = it }
      }

    /** 前面として数えないパッケージ（通知シェード・キーボードなど）。 */
    private val IGNORED = setOf("com.android.systemui", "android")
  }

  val context: Context = context
  val prefs = Prefs(context)
  val stateStore = PrefsStore(context)
  val tracker = Tracker(stateStore)
  val link = Link(this)
  val appVersion = "0.1.0"

  private val main = Handler(Looper.getMainLooper())

  /** Accessibility Service がつながっている間だけ入る。 */
  @Volatile var guard: GuardService? = null
  private var overlay: Overlay? = null
  private var imePackages: Set<String>? = null

  data class Flags(val accessibility: Boolean, val deviceAdmin: Boolean, val batteryUnrestricted: Boolean)

  fun flags(): Flags {
    val dpm = context.getSystemService(Context.DEVICE_POLICY_SERVICE) as DevicePolicyManager
    val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
    return Flags(
      accessibility = guard != null,
      deviceAdmin = dpm.isAdminActive(ComponentName(context, AdminReceiver::class.java)),
      batteryUnrestricted = pm.isIgnoringBatteryOptimizations(context.packageName),
    )
  }

  // ---- 開始・終了 ----

  fun start() {
    link.start()
    reevaluate()
  }

  fun stop(bye: Boolean) {
    link.stop(bye)
    main.post {
      main.removeCallbacks(wake)
      overlay?.hideAll()
    }
  }

  // ---- Accessibility Service とのつなぎ ----

  fun attachGuard(service: GuardService) {
    guard = service
    main.post {
      overlay?.hideAll()
      overlay = Overlay(service, this)
      reevaluate()
    }
    link.heartbeatNow()
  }

  fun detachGuard(service: GuardService) {
    if (guard !== service) return
    guard = null
    main.post {
      overlay?.hideAll()
      overlay = null
    }
    // 保護が外れたことを、次のハートビートを待たずに伝える（サーバーが状態の変化から友達に通知する）
    link.heartbeatNow()
  }

  // ---- 前面のアプリ ----

  private fun imes(): Set<String> =
    imePackages ?: run {
      val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
      imm.enabledInputMethodList.map { it.packageName }.toSet().also { imePackages = it }
    }

  /** このパッケージの画面の変化を、前面の変化として扱うか。 */
  fun countsAsForeground(pkg: String): Boolean {
    if (pkg in IGNORED || pkg in imes()) return false
    // 自分のオーバーレイが出ている間の、自分の画面の変化は前面ではない
    if (pkg == context.packageName && overlay?.visible == true) return false
    return true
  }

  /** 前面が変わった。画面オフは null。 */
  fun onForeground(pkg: String?) {
    tracker.onForeground(pkg?.let { "android:$it" }, System.currentTimeMillis())
    reevaluate()
  }

  fun onScreenOff() = onForeground(null)

  /** 画面が点いたとき、いまの前面を調べ直す。 */
  fun refreshForeground() {
    val pkg = guard?.rootInActiveWindow?.packageName?.toString()
    if (pkg != null && countsAsForeground(pkg)) onForeground(pkg)
  }

  // ---- 判定と表示 ----

  private val wake = Runnable { doEvaluate() }

  fun reevaluate() {
    main.post { doEvaluate() }
  }

  private fun doEvaluate() {
    main.removeCallbacks(wake)
    val now = System.currentTimeMillis()
    val d = tracker.decide(now)

    for ((graceKey, _) in d.newGraces) {
      val ruleId = graceKey.substringBefore('|')
      val name = tracker.ruleName(ruleId) ?: "制限"
      val app = tracker.currentKey?.removePrefix("android:")?.let(::appLabel) ?: ""
      link.report("limit_reached", "「$name」の制限に達しました（$app）")
    }

    val ruleName = when (val v = d.verdict) {
      is Verdict.Grace -> tracker.ruleName(v.ruleId)
      is Verdict.Block -> tracker.ruleName(v.ruleId)
      else -> null
    }
    overlay?.apply(d.verdict, ruleName)
    d.nextWakeup?.let { main.postDelayed(wake, maxOf(500L, it - now + 50)) }
  }

  fun onPolicy() = reevaluate()

  fun appLabel(pkg: String): String = try {
    val pm = context.packageManager
    pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString()
  } catch (_: Exception) {
    pkg
  }

  // ---- 本人の操作 ----

  /** 本人が一時解除した。`reason` は友達への通知の本文になる。 */
  fun unlock(reason: String) {
    tracker.startUnlock(reason.trim(), System.currentTimeMillis())
    link.unlockNow()
    reevaluate()
  }

  fun goHome() {
    guard?.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME)
  }

  // ---- サーバーからのイベント ----

  fun onEvent(event: JSONObject) {
    tracker.noteEvent(event.getString("id"))
    val user = event.getJSONObject("user")
    if (user.getString("id") == prefs.userId) return // 自分のことは通知しない
    Notices.friendEvent(context, event)
  }
}
