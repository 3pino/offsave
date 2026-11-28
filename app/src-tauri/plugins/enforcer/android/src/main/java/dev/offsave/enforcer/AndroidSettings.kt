package dev.offsave.enforcer

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings

/**
 * このアプリの Android の設定の、各項目へ直接飛ぶ。「アプリ情報」の画面（無効化・アンインストール・強制停止のボタンがある）は経由しない。
 *
 * 項目の名前は、画面（`AndroidSettings.svelte`）と対応する。
 */
object AndroidSettings {
  /** `kind` の設定画面を開くための intent の候補（上から試す）。知らない `kind` は null。 */
  fun intents(kind: String, pkg: String): List<Intent>? {
    val uri = Uri.parse("package:$pkg")
    return when (kind) {
      "notifications" -> listOf(
        Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, pkg),
      )
      // 権限（Android 13 以降は、通知だけが並ぶ）
      "permissions" -> listOf(
        Intent("android.intent.action.MANAGE_APP_PERMISSIONS").putExtra("android.intent.extra.PACKAGE_NAME", pkg),
      )
      // モバイルデータの使用量（バックグラウンドのデータ・データセーバーの例外）
      "data" -> listOf(
        Intent("android.settings.IGNORE_BACKGROUND_DATA_RESTRICTIONS_SETTINGS").setData(uri),
      )
      // 電池の最適化の一覧。アプリごとの「電池の使用状況」の詳細（VIEW_ADVANCED_POWER_USAGE_DETAIL）は、
      // アプリ情報と同じ、アンインストール・強制停止のボタンがあるので使わない
      "battery" -> listOf(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
      // デフォルトで開く（Android 12 以降）
      "open_by_default" -> listOf(Intent("android.settings.APP_OPEN_BY_DEFAULT_SETTINGS").setData(uri))
      else -> null
    }
  }

  /** 開けたら true。このデバイスに無い項目・知らない `kind` は false。 */
  fun open(context: Context, kind: String): Boolean {
    for (intent in intents(kind, context.packageName) ?: return false) {
      try {
        context.startActivity(intent)
        return true
      } catch (_: ActivityNotFoundException) {
      } catch (_: SecurityException) {
      }
    }
    return false
  }
}
