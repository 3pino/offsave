package dev.offsave.enforcer

import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.os.SystemClock
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
  val pause = Pause(this)
  val appVersion = "0.1.0"

  private val main = Handler(Looper.getMainLooper())

  /** Accessibility Service がつながっている間だけ入る。 */
  @Volatile var guard: GuardService? = null
  private var overlay: Overlay? = null
  private var imePackages: Set<String>? = null

  // ---- 時計 ----
  // サーバーの時刻を基準にし、端末を再起動しない限り経過時間（elapsedRealtime）で進める。
  // 端末の時計を動かして日付をごまかすのを防ぎ、オフラインでも横ばいになる。
  @Volatile private var anchorServer = 0L
  @Volatile private var anchorElapsed = 0L
  private var lastSkewReport = 0L

  fun now(): Long {
    val e = SystemClock.elapsedRealtime()
    return if (anchorServer > 0 && e >= anchorElapsed) anchorServer + (e - anchorElapsed) else System.currentTimeMillis()
  }

  data class Flags(val accessibility: Boolean, val deviceAdmin: Boolean, val batteryUnrestricted: Boolean, val paused: Boolean)

  fun flags(): Flags {
    val dpm = context.getSystemService(Context.DEVICE_POLICY_SERVICE) as DevicePolicyManager
    val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
    return Flags(
      accessibility = guard != null,
      deviceAdmin = dpm.isAdminActive(ComponentName(context, AdminReceiver::class.java)),
      batteryUnrestricted = pm.isIgnoringBatteryOptimizations(context.packageName),
      paused = pause.pausedFlag,
    )
  }

  // ---- 開始・終了 ----

  fun start() {
    link.start()
    // 一時停止の途中でプロセスが再開したときは、続き（戻すこと）を引き受ける
    main.post { pause.resume() }
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

  fun imes(): Set<String> =
    imePackages ?: run {
      val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
      imm.enabledInputMethodList.map { it.packageName }.toSet().also { imePackages = it }
    }

  /** このパッケージの画面の変化を、前面の変化として扱うか。 */
  fun countsAsForeground(pkg: String): Boolean {
    if (pkg in IGNORED || pkg in imes()) return false
    // オーバーレイを出し入れした直後の、自分の画面の変化は、オーバーレイ自身のものなので前面ではない。
    // それ以外の自分のアプリ（offsave の画面）への切り替えは、普通のアプリとして扱う。
    if (pkg == context.packageName && overlay?.changedRecently() == true) return false
    return true
  }

  /** 前面が変わった。画面オフは null。 */
  fun onForeground(pkg: String?) {
    // 決済アプリなどが前面に来たら、何より先に Accessibility を外す（アプリが確かめるより前に）
    pause.onForeground(pkg)
    tracker.onForeground(pkg?.let { "android:$it" }, now())
    guard?.watchBrowser(pkg?.takeIf { Browsers.supported(it) })
    if (pkg != null) checkUnsupportedBrowser(pkg)
    reevaluate()
  }

  /** ブラウザで見ているサイトが変わった（ホスト名だけ）。 */
  fun onDomain(host: String?) {
    tracker.onDomain(host, now())
    reevaluate()
  }

  // ---- 対応していないブラウザ ----

  private val browserPackages: Set<String> by lazy {
    val intent = Intent(Intent.ACTION_VIEW, Uri.parse("http://example.com")).addCategory(Intent.CATEGORY_BROWSABLE)
    context.packageManager.queryIntentActivities(intent, PackageManager.MATCH_ALL)
      .map { it.activityInfo.packageName }.toSet()
  }
  private val unsupportedReportedAt = HashMap<String, Long>()

  /** URL を読めないブラウザは、サイトの制限が効かないので、友達に知らせる（同じブラウザは 1 時間に 1 回）。 */
  private fun checkUnsupportedBrowser(pkg: String) {
    if (Browsers.supported(pkg) || pkg !in browserPackages || pkg == context.packageName) return
    val t = SystemClock.elapsedRealtime()
    if (t - (unsupportedReportedAt[pkg] ?: -3_600_000L) < 3_600_000L) return
    unsupportedReportedAt[pkg] = t
    link.report("unsupported_browser", "対応していないブラウザ「${appLabel(pkg)}」を使いました（サイトの制限は効きません）")
  }

  fun onScreenOff() {
    pause.onScreenOff()
    onForeground(null)
  }

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
    val now = now()
    val d = tracker.decide(now)

    // 制限に達したとき、どのアプリ・サイトかを友達に伝える。使えない時間帯は、決まった時刻になるだけなので伝えない（合計時間だけ）。
    // 同じ制限は、本人がボタンを押すまで判定のたびに「新しい」と出るので、一度だけ伝える
    for ((graceKey, _) in d.newGraces) {
      if (tracker.limitKindOf(graceKey) == BlockKind.WINDOW) continue
      if (!reportedLimits.add(graceKey)) continue
      val ruleId = graceKey.substringBefore('|')
      val name = tracker.ruleName(ruleId) ?: "制限"
      val subject = tracker.graceSubjects[graceKey] ?: tracker.foregroundKey ?: ""
      val what = when {
        subject.startsWith("domain:") -> subject.removePrefix("domain:")
        subject.startsWith("android:") -> appLabel(subject.removePrefix("android:"))
        else -> subject
      }
      link.report("limit_reached", "「$name」の制限に達しました（$what）")
    }

    val ruleName = when (val v = d.verdict) {
      is Verdict.Grace -> tracker.ruleName(v.ruleId)
      is Verdict.Block -> tracker.ruleName(v.ruleId)
      else -> null
    }
    // 新しく制限に達した（猶予をまだ始めていない）ときは、全画面で知らせる。「+60秒」を押すまでは、何秒でも待つ
    val reached = d.newGraces.firstOrNull()?.first
    if (reached != null && d.verdict !is Verdict.Block) {
      val name = tracker.ruleName(reached.substringBefore('|')) ?: "制限"
      val daily = d.newGraces.any { tracker.limitKindOf(it.first) != BlockKind.WINDOW }
      Notices.cancelGraceCountdown(context)
      overlay?.showNotice(
        if (daily) "1日の制限に達しました" else "使えない時間帯になりました",
        if (daily) "「$name」の 1 日の上限に達しました。" else "「$name」の使えない時間帯です。",
      ) { acceptLimit() }
    } else {
      overlay?.apply(d.verdict, ruleName)
      // 猶予の間（60 秒）は、画面は占有せず、残り時間を通知で示す
      val v = d.verdict
      if (v is Verdict.Grace) {
        Notices.graceCountdown(context, "「${ruleName ?: "制限"}」。残り時間が 0 になると使えなくなります", maxOf(1_000L, v.until - now))
      } else {
        Notices.cancelGraceCountdown(context)
      }
    }
    d.nextWakeup?.let { main.postDelayed(wake, maxOf(500L, it - now + 50)) }
  }

  private val reportedLimits = HashSet<String>()

  /** 「+60秒」を押した。ここから 60 秒を数え始め、全画面を閉じて通知だけにする。 */
  private fun acceptLimit() {
    tracker.startPendingGraces(now())
    doEvaluate()
  }

  fun onPolicy() {
    tracker.policy?.serverTime?.let { server ->
      val skew = System.currentTimeMillis() - server
      anchorServer = server
      anchorElapsed = SystemClock.elapsedRealtime()
      val t = SystemClock.elapsedRealtime()
      if (kotlin.math.abs(skew) > 3 * MIN_MS && t - lastSkewReport > 30 * MIN_MS) {
        lastSkewReport = t
        val min = skew / MIN_MS
        link.report("clock_skew", "デバイスの時計がサーバーより ${if (min >= 0) "" else "-"}${kotlin.math.abs(min)} 分ずれています（制限はサーバーの時刻で数えています）")
      }
    }
    reevaluate()
  }

  fun appLabel(pkg: String): String = try {
    val pm = context.packageManager
    pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString()
  } catch (_: Exception) {
    pkg
  }

  // ---- 本人の操作 ----

  /** 本人が一時解除した。`reason` は友達への通知の本文になる。 */
  fun unlock(reason: String) {
    tracker.startUnlock(reason.trim(), now())
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
