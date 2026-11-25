package dev.offsave.enforcer

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** 起動後とアプリの更新後に、常駐サービスを動かし直す。 */
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action != Intent.ACTION_BOOT_COMPLETED && intent.action != Intent.ACTION_MY_PACKAGE_REPLACED) return
    if (Prefs(context).configured) AgentService.start(context)
  }
}
