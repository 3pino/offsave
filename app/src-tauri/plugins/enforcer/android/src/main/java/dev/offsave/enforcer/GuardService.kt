package dev.offsave.enforcer

import android.accessibilityservice.AccessibilityService
import android.content.Intent
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo

/**
 * 前面のアプリを見張る Accessibility Service。
 *
 * - 窓が切り替わるたびに前面のアプリを記録する（ポーリングしない。電池のため）。
 * - 設定画面で offsave の無効化・アンインストールの画面を開いたら、友達に知らせてホームに戻す。
 * - 制限の表示（オーバーレイ）は、この Service の窓として出す。
 */
class GuardService : AccessibilityService() {
  private var lastTamperAt = 0L

  override fun onServiceConnected() {
    val agent = Agent.get(this)
    agent.attachGuard(this)
    // 設定の済んだデバイスなら、通信のサービスも動かす
    if (agent.prefs.configured) AgentService.start(this)
  }

  override fun onAccessibilityEvent(event: AccessibilityEvent) {
    if (event.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
    val pkg = event.packageName?.toString() ?: return
    val agent = Agent.get(this)

    if (pkg in SETTINGS_PACKAGES) checkTamper(agent)

    if (agent.countsAsForeground(pkg)) agent.onForeground(pkg)
  }

  override fun onInterrupt() {}

  override fun onUnbind(intent: Intent?): Boolean {
    Agent.get(this).detachGuard(this)
    return super.onUnbind(intent)
  }

  override fun onDestroy() {
    Agent.get(this).detachGuard(this)
    super.onDestroy()
  }

  // ---- 設定画面の見張り ----

  private fun checkTamper(agent: Agent) {
    if (!agent.prefs.guardSettings) return
    val now = System.currentTimeMillis()
    if (now < agent.prefs.setupUntil) return // 権限の設定の案内の最中
    val root = rootInActiveWindow ?: return
    val texts = ArrayList<String>()
    collectTexts(root, texts, 400)
    if (!looksLikeOurDangerousPage(texts, packageName.let { agent.appLabel(it) })) return

    if (now - lastTamperAt > 30_000) {
      lastTamperAt = now
      agent.link.report("tamper_attempt", "設定画面で offsave の無効化・アンインストールの画面を開きました")
    }
    performGlobalAction(GLOBAL_ACTION_HOME)
  }

  private fun collectTexts(node: AccessibilityNodeInfo, out: MutableList<String>, limit: Int) {
    if (out.size >= limit) return
    node.text?.toString()?.takeIf { it.isNotBlank() }?.let { out.add(it) }
    node.contentDescription?.toString()?.takeIf { it.isNotBlank() }?.let { out.add(it) }
    for (i in 0 until node.childCount) {
      val child = node.getChild(i) ?: continue
      collectTexts(child, out, limit)
    }
  }

  companion object {
    private val SETTINGS_PACKAGES = setOf(
      "com.android.settings",
      "com.google.android.packageinstaller",
      "com.android.packageinstaller",
      "com.samsung.android.settings",
    )

    private val DANGEROUS = listOf(
      "アンインストール", "uninstall", "強制停止", "force stop",
      "無効にする", "無効化", "deactivate", "disable", "停止",
    )

    /** アプリ名（offsave）を含み、無効化・アンインストールの言葉もある画面か。 */
    fun looksLikeOurDangerousPage(texts: List<String>, appLabel: String): Boolean {
      val lower = texts.map { it.lowercase() }
      val name = appLabel.lowercase()
      val mentionsUs = lower.any { it.contains("offsave") || it.contains(name) }
      return mentionsUs && lower.any { t -> DANGEROUS.any { t.contains(it) } }
    }
  }
}
