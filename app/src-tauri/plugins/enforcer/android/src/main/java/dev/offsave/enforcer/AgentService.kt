package dev.offsave.enforcer

import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.content.ContextCompat

/**
 * 常駐するフォアグラウンドサービス。サーバーとの WebSocket を保ち、ハートビートを送る。
 * 画面のオン・オフは、ここで受けて前面の記録に反映する（画面オフの間は使っていない）。
 */
class AgentService : Service() {
  companion object {
    @Volatile var running = false
      private set

    fun start(context: Context) {
      ContextCompat.startForegroundService(context, Intent(context, AgentService::class.java))
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, AgentService::class.java))
    }
  }

  private val screenReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      val agent = Agent.get(context)
      when (intent.action) {
        Intent.ACTION_SCREEN_OFF -> agent.onScreenOff()
        Intent.ACTION_SCREEN_ON, Intent.ACTION_USER_PRESENT -> agent.refreshForeground()
      }
    }
  }

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val notification = Notices.serviceNotification(this)
    if (Build.VERSION.SDK_INT >= 34) {
      startForeground(Notices.SERVICE_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
    } else {
      startForeground(Notices.SERVICE_ID, notification)
    }

    val agent = Agent.get(this)
    if (!agent.prefs.configured) {
      stopSelf()
      return START_NOT_STICKY
    }
    if (!running) {
      running = true
      registerReceiver(
        screenReceiver,
        IntentFilter().apply {
          addAction(Intent.ACTION_SCREEN_OFF)
          addAction(Intent.ACTION_SCREEN_ON)
          addAction(Intent.ACTION_USER_PRESENT)
        },
      )
    }
    agent.start()
    return START_STICKY
  }

  override fun onDestroy() {
    if (running) {
      running = false
      runCatching { unregisterReceiver(screenReceiver) }
      // 意図した停止（clear）は plugin 側で bye を送る。ここは OS に止められた場合もあるので bye は送らない
      Agent.get(this).stop(bye = false)
    }
    super.onDestroy()
  }
}
