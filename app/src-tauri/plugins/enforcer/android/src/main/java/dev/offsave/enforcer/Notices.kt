package dev.offsave.enforcer

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationChannelGroup
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
  const val SERVICE_ID = 1

  /** 友達のイベントの通知チャンネル。種類ごとに、端末の設定で音・重要度・オンオフを変えられる。 */
  class EventChannel(val id: String, val name: String, val description: String, val importance: Int, val kinds: Set<String>)

  val EVENT_CHANNELS = listOf(
    EventChannel(
      "unlock", "制限の一時解除", "友達が制限を一時解除したとき",
      NotificationManager.IMPORTANCE_HIGH, setOf("temp_unlock"),
    ),
    EventChannel(
      "limit", "制限に達した", "友達が 1 日の合計時間の上限に達したとき",
      NotificationManager.IMPORTANCE_DEFAULT, setOf("limit_reached"),
    ),
    EventChannel(
      "protection", "監視の無効化・不審な操作", "監視が外れた・無効化やアンインストールを試した・時計がずれた・対応外のブラウザを使ったとき",
      NotificationManager.IMPORTANCE_HIGH,
      setOf("protection_disabled", "tamper_attempt", "clock_skew", "unsupported_browser"),
    ),
    EventChannel(
      "protection_pause", "監視の一時停止・再開", "決済アプリなどのために監視を一時停止した・再開したとき",
      NotificationManager.IMPORTANCE_DEFAULT, setOf("protection_paused", "protection_restored"),
    ),
    EventChannel(
      "connection", "接続の状態", "友達のデバイスとの連絡が途絶えた・再びつながったとき",
      NotificationManager.IMPORTANCE_DEFAULT, setOf("device_offline", "device_online"),
    ),
    EventChannel(
      "devices", "デバイスの追加・削除", "友達のデバイスが追加・削除されたとき",
      NotificationManager.IMPORTANCE_DEFAULT, setOf("device_added", "device_removed"),
    ),
    EventChannel(
      "rules", "ルール・設定の変更", "友達のルールや設定が変わったとき",
      NotificationManager.IMPORTANCE_DEFAULT, setOf("rule_changed", "settings_changed"),
    ),
    EventChannel(
      "friendship", "友達の追加・解消", "友達になった・解消されたとき",
      NotificationManager.IMPORTANCE_DEFAULT, setOf("friend_added", "friend_removed"),
    ),
  )
  private const val CHANNEL_OTHER = "other"
  private const val CHANNEL_GRACE = "grace"
  private const val GRACE_ID = 2
  private const val GROUP_EVENTS = "events"

  /** イベントの種類に対応するチャンネルの ID。 */
  fun channelOf(kind: String): String = EVENT_CHANNELS.firstOrNull { kind in it.kinds }?.id ?: CHANNEL_OTHER

  fun ensureChannels(context: Context) {
    val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    nm.createNotificationChannel(
      NotificationChannel(CHANNEL_SERVICE, "動作中の表示", NotificationManager.IMPORTANCE_MIN).apply {
        description = "offsave が動いていることを示します（消さないでください）"
        setShowBadge(false)
      },
    )
    nm.createNotificationChannel(
      NotificationChannel(CHANNEL_GRACE, "制限までの残り時間", NotificationManager.IMPORTANCE_HIGH).apply {
        description = "制限に達してから使えなくなるまでの 60 秒を、通知で知らせます（画面は占有しません）"
        setShowBadge(false)
      },
    )
    // 以前は「友達の通知」チャンネル 1 つだった。種類ごとに分けたので消す
    nm.deleteNotificationChannel("friends")
    nm.createNotificationChannelGroup(NotificationChannelGroup(GROUP_EVENTS, "友達の通知"))
    for (c in EVENT_CHANNELS) {
      nm.createNotificationChannel(
        NotificationChannel(c.id, c.name, c.importance).apply {
          description = c.description
          group = GROUP_EVENTS
        },
      )
    }
    nm.createNotificationChannel(
      NotificationChannel(CHANNEL_OTHER, "その他", NotificationManager.IMPORTANCE_DEFAULT).apply { group = GROUP_EVENTS },
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
    "protection_disabled" -> "監視が無効になりました"
    "protection_restored" -> "監視が有効になりました"
    "tamper_attempt" -> "無効化・アンインストールを試みました"
    "limit_reached" -> "制限に達しました"
    "friend_added" -> "友達になりました"
    "friend_removed" -> "友達を解消しました"
    "settings_changed" -> "設定を変更しました"
    "rule_changed" -> "ルールが変わりました"
    "unsupported_browser" -> "対応していないブラウザを使いました"
    "clock_skew" -> "時計がずれています"
    "protection_paused" -> "監視を一時停止・再開しました"
    else -> kind
  }

  /** 猶予の残り時間。通知だけで示す（画面の一部を占有しない）。`remainingMs` 後に自動で消える。 */
  fun graceCountdown(context: Context, text: String, remainingMs: Long) {
    if (android.os.Build.VERSION.SDK_INT >= 33 &&
      context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) return
    ensureChannels(context)
    val n = NotificationCompat.Builder(context, CHANNEL_GRACE)
      .setSmallIcon(context.applicationInfo.icon)
      .setContentTitle("制限に達しました")
      .setContentText(text)
      .setWhen(System.currentTimeMillis() + remainingMs)
      .setShowWhen(true)
      .setUsesChronometer(true)
      .setChronometerCountDown(true)
      .setTimeoutAfter(remainingMs)
      .setOnlyAlertOnce(true)
      .setCategory(NotificationCompat.CATEGORY_STATUS)
      .setPriority(NotificationCompat.PRIORITY_HIGH)
      .setContentIntent(openApp(context))
      .build()
    NotificationManagerCompat.from(context).notify(GRACE_ID, n)
  }

  fun cancelGraceCountdown(context: Context) {
    NotificationManagerCompat.from(context).cancel(GRACE_ID)
  }

  fun friendEvent(context: Context, event: JSONObject) {
    if (android.os.Build.VERSION.SDK_INT >= 33 &&
      context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) return
    ensureChannels(context)
    val who = event.getJSONObject("user").getString("display_name")
    val kind = event.getString("kind")
    val body = event.getString("message")
    val n = NotificationCompat.Builder(context, channelOf(kind))
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
