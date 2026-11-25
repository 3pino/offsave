package dev.offsave.enforcer

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import org.json.JSONObject

/** 通知。常駐の通知と、友達についてのイベントの通知。 */
object Notices {
  const val CHANNEL_SERVICE = "service"
  const val CHANNEL_FRIENDS = "friends"
  const val SERVICE_ID = 1

  fun ensureChannels(context: Context) {
    val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    nm.createNotificationChannel(
      NotificationChannel(CHANNEL_SERVICE, "動作中の表示", NotificationManager.IMPORTANCE_MIN).apply {
        description = "offsave が動いていることを示します（消さないでください）"
        setShowBadge(false)
      },
    )
    nm.createNotificationChannel(
      NotificationChannel(CHANNEL_FRIENDS, "友達の通知", NotificationManager.IMPORTANCE_HIGH).apply {
        description = "友達の制限の解除や、保護の無効化などを知らせます"
      },
    )
  }

  private fun openApp(context: Context): PendingIntent {
    val intent = context.packageManager.getLaunchIntentForPackage(context.packageName)
      ?: Intent()
    return PendingIntent.getActivity(
      context, 0, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
  }

  fun serviceNotification(context: Context): Notification {
    ensureChannels(context)
    return NotificationCompat.Builder(context, CHANNEL_SERVICE)
      .setSmallIcon(context.applicationInfo.icon)
      .setContentTitle("offsave")
      .setContentText("制限を見守っています")
      .setOngoing(true)
      .setPriority(NotificationCompat.PRIORITY_MIN)
      .setContentIntent(openApp(context))
      .build()
  }

  private fun label(kind: String): String = when (kind) {
    "temp_unlock" -> "制限を一時解除しました"
    "device_offline" -> "連絡が途絶えました"
    "device_online" -> "再び接続しました"
    "device_added" -> "デバイスが追加されました"
    "device_removed" -> "デバイスが削除されました"
    "protection_disabled" -> "保護が無効になりました"
    "protection_restored" -> "保護が有効になりました"
    "tamper_attempt" -> "無効化・アンインストールを試みました"
    "limit_reached" -> "制限に達しました"
    "friend_added" -> "友達になりました"
    "friend_removed" -> "友達を解消しました"
    "settings_changed" -> "設定を変更しました"
    "rule_changed" -> "ルールが変わりました"
    else -> kind
  }

  fun friendEvent(context: Context, event: JSONObject) {
    if (android.os.Build.VERSION.SDK_INT >= 33 &&
      context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) return
    ensureChannels(context)
    val who = event.getJSONObject("user").getString("display_name")
    val kind = event.getString("kind")
    val body = event.getString("message")
    val n = NotificationCompat.Builder(context, CHANNEL_FRIENDS)
      .setSmallIcon(context.applicationInfo.icon)
      .setContentTitle("$who: ${label(kind)}")
      .setContentText(body)
      .setStyle(NotificationCompat.BigTextStyle().bigText(body))
      .setAutoCancel(true)
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setContentIntent(openApp(context))
      .build()
    // 同じ ID で上書きしないように、イベントの ID から作る
    val id = (event.getString("id").hashCode() and 0x7fffffff) + 100
    NotificationManagerCompat.from(context).notify(id, n)
  }
}
