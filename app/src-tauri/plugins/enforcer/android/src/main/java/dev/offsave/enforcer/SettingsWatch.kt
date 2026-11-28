package dev.offsave.enforcer

import android.app.NotificationManager
import android.content.Context
import android.net.ConnectivityManager
import android.os.PowerManager
import org.json.JSONObject

/**
 * 友達の見守りに必要な Android の設定が、本人によって off にされたかを見る。
 *
 * 設定画面を開いただけでは通知しない。on だったものが off になったときだけ、友達に知らせる
 * （off から on に戻ったときは、黙って見張りを戻す）。Accessibility とデバイス管理者は、ハートビートの状態（`DeviceFlags`）で別に扱う。
 */
class SettingsWatch(private val agent: Agent) {
  companion object {
    /** 見張る項目: id → 友達への通知に出す名前 */
    val LABELS = linkedMapOf(
      "notifications" to "通知",
      "battery" to "電池の使用状況（制限なし）",
      "background_data" to "バックグラウンドのデータ",
      "usage_access" to "使用状況へのアクセス",
    )

    /** 前回 on で、いま off になった項目の id（前回が分からない項目は数えない）。 */
    fun lost(before: Map<String, Boolean>, now: Map<String, Boolean>): List<String> =
      LABELS.keys.filter { before[it] == true && now[it] == false }

    fun encode(state: Map<String, Boolean>): String =
      JSONObject(state.mapValues { it.value }).toString()

    fun decode(json: String?): Map<String, Boolean> {
      if (json.isNullOrEmpty()) return emptyMap()
      return try {
        val o = JSONObject(json)
        o.keys().asSequence().associateWith { o.getBoolean(it) }
      } catch (_: Exception) {
        emptyMap()
      }
    }
  }

  private val context get() = agent.context

  /** いまの状態（on が true）。 */
  fun current(): Map<String, Boolean> {
    val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
    val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    return mapOf(
      "notifications" to nm.areNotificationsEnabled(),
      "battery" to pm.isIgnoringBatteryOptimizations(context.packageName),
      // バックグラウンドのデータが止められていない（データセーバーが off か、このアプリが例外）
      "background_data" to (cm.restrictBackgroundStatus != ConnectivityManager.RESTRICT_BACKGROUND_STATUS_ENABLED),
      "usage_access" to agent.pause.hasUsageAccess,
    )
  }

  /** 状態を調べ、off にされたものがあれば友達に知らせる。 */
  @Synchronized
  fun check() {
    if (!agent.prefs.configured) return
    val now = current()
    val before = decode(agent.prefs.watchedState)
    // 前回の状態が無いとき（初回）は、いまの状態を基準にするだけ
    if (before.isNotEmpty()) {
      val lost = lost(before, now).map { LABELS.getValue(it) }
      if (lost.isNotEmpty()) {
        agent.link.report("protection_disabled", "Android の設定で、offsave の「${lost.joinToString("」「")}」が off にされました")
      }
    }
    val saved = encode(now)
    if (saved != agent.prefs.watchedState) agent.prefs.watchedState = saved
  }
}
