package dev.offsave.enforcer

import android.Manifest
import android.app.Activity
import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.core.app.ActivityCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.Executors
import org.json.JSONObject

// 命令の引数。Tauri が JSON から組み立てる
@InvokeArg
class ConfigureArgs {
  lateinit var serverUrl: String
  var devUser: String? = null
  lateinit var deviceId: String
  lateinit var token: String
  lateinit var userId: String
  var guardSettings: Boolean = true
}

@InvokeArg
class AppIconArgs {
  lateinit var pkg: String
}

/**
 * WebView（Svelte の画面）から、Android の強制の仕組みを操作する入口。
 * 命令の名前と権限は build.rs と permissions/default.toml に対応する。
 */
@TauriPlugin
class EnforcerPlugin(private val activity: Activity) : Plugin(activity) {
  private val agent get() = Agent.get(activity)

  /** 重い命令（アプリ一覧・アイコン）は画面のスレッドではやらず、ここで 1 つずつやる（入力やスクロールを止めない）。 */
  private val worker = Executors.newSingleThreadExecutor { r -> Thread(r, "offsave-worker").apply { isDaemon = true } }

  /** 権限の設定のために設定画面を開く間は、設定画面の見張りを止める。 */
  private fun enterSetup() {
    agent.prefs.setupUntil = System.currentTimeMillis() + 3 * 60_000
  }

  @Command
  fun configure(invoke: Invoke) {
    val args = invoke.parseArgs(ConfigureArgs::class.java)
    val prefs = agent.prefs
    prefs.serverUrl = args.serverUrl.trim()
    prefs.devUser = args.devUser?.trim()?.takeIf { it.isNotEmpty() }
    prefs.deviceId = args.deviceId
    prefs.token = args.token
    prefs.userId = args.userId
    prefs.guardSettings = args.guardSettings
    AgentService.start(activity)
    invoke.resolve(statusObject())
  }

  /** 連携を解除する（友達に「終了」を送ってから、設定と記録を消す）。 */
  @Command
  fun clear(invoke: Invoke) {
    agent.stop(bye = true)
    AgentService.stop(activity)
    agent.prefs.clear()
    agent.stateStore.clear()
    invoke.resolve(statusObject())
  }

  @Command
  fun status(invoke: Invoke) {
    invoke.resolve(statusObject())
  }

  private fun statusObject(): JSObject {
    val a = agent
    val f = a.flags()
    val now = a.now()
    val o = JSObject()
    o.put("configured", a.prefs.configured)
    o.put("accessibility", f.accessibility)
    o.put("deviceAdmin", f.deviceAdmin)
    o.put("batteryUnrestricted", f.batteryUnrestricted)
    o.put(
      "notifications",
      android.os.Build.VERSION.SDK_INT < 33 ||
        activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED,
    )
    o.put("serviceRunning", AgentService.running)
    o.put("connected", a.link.connected)
    o.put("lastError", a.link.lastError ?: JSONObject.NULL)
    o.put("lastPolicyAt", a.link.lastPolicyAt)
    o.put("unlockUntil", a.tracker.unlockUntil(now))
    o.put("foreground", a.tracker.foregroundKey ?: JSONObject.NULL)
    o.put("ruleCount", a.tracker.policy?.rules?.size ?: 0)
    return o
  }

  @Command
  fun openAccessibilitySettings(invoke: Invoke) {
    enterSetup()
    activity.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
    invoke.resolve()
  }

  @Command
  fun requestDeviceAdmin(invoke: Invoke) {
    enterSetup()
    val intent = Intent(DevicePolicyManager.ACTION_ADD_DEVICE_ADMIN)
      .putExtra(DevicePolicyManager.EXTRA_DEVICE_ADMIN, ComponentName(activity, AdminReceiver::class.java))
      .putExtra(
        DevicePolicyManager.EXTRA_ADD_EXPLANATION,
        "有効にすると、友達への通知なしに offsave をアンインストールできなくなります。",
      )
    activity.startActivity(intent)
    invoke.resolve()
  }

  @Command
  fun requestBatteryExemption(invoke: Invoke) {
    enterSetup()
    val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS)
      .setData(Uri.parse("package:${activity.packageName}"))
    activity.startActivity(intent)
    invoke.resolve()
  }

  @Command
  fun requestNotifications(invoke: Invoke) {
    if (android.os.Build.VERSION.SDK_INT >= 33) {
      ActivityCompat.requestPermissions(activity, arrayOf(Manifest.permission.POST_NOTIFICATIONS), 7001)
    }
    invoke.resolve()
  }

  /** アプリ情報（Android 13 以降で「制限された設定」を許可するのに使う）。 */
  @Command
  fun openAppDetails(invoke: Invoke) {
    enterSetup()
    val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
      .setData(Uri.parse("package:${activity.packageName}"))
    activity.startActivity(intent)
    invoke.resolve()
  }

  /** アプリのアイコン（小さな PNG を base64 にしたもの）。取れなければ png は空。選択の一覧の行を見せるときに、必要な分だけ呼ぶ。 */
  @Command
  fun appIcon(invoke: Invoke) {
    val args = invoke.parseArgs(AppIconArgs::class.java)
    worker.execute {
      val png = try { Icons.pngBase64(activity, args.pkg) } catch (e: Exception) { null }
      invoke.resolve(JSObject().put("png", png ?: ""))
    }
  }

  /** このデバイスの起動できるアプリ（ルールの対象を選ぶのに使う）。 */
  @Command
  fun installedApps(invoke: Invoke) {
    worker.execute {
      val pm = activity.packageManager
      val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
      val apps = JSArray()
      // offsave 自身も含める（非表示の設定やルールの対象に選べるように）
      // ホーム画面のアプリ（launcher3 など）も含める
      val home = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME)
      (pm.queryIntentActivities(launcher, 0) + pm.queryIntentActivities(home, 0))
        .map { it.activityInfo.packageName to it.loadLabel(pm).toString() }
        .distinctBy { it.first }
        .sortedBy { it.second.lowercase() }
        .forEach { (pkg, label) ->
          apps.put(JSObject().put("package", pkg).put("label", label))
        }
      invoke.resolve(JSObject().put("apps", apps))
    }
  }
}
