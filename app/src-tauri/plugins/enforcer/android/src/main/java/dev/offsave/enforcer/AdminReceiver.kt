package dev.offsave.enforcer

import android.app.admin.DeviceAdminReceiver
import android.content.Context
import android.content.Intent

/**
 * デバイス管理者。有効にしている間は、offsave をアンインストールできない
 * （先に管理者を無効にする必要があり、その操作は友達に通知される）。
 */
class AdminReceiver : DeviceAdminReceiver() {
  override fun onDisableRequested(context: Context, intent: Intent): CharSequence =
    "offsave のデバイス管理者を無効にすると、友達に通知されます。"

  override fun onDisabled(context: Context, intent: Intent) {
    // 次のハートビートを待たずに伝える（状態の変化は、サーバーが友達に通知する）
    Agent.get(context).link.heartbeatNow()
  }
}
