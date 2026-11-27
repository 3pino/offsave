package dev.offsave.enforcer

import android.os.Handler
import android.os.HandlerThread
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * サーバーの WebSocket（`/api/ws/device`）。
 *
 * 接続している間は 60 秒ごとにハートビートを送る。ハートビートには保護の状態と前面のアプリを載せ、
 * 続いている利用区間はここで区切って送る。返事の方針で、制限を更新する。
 * 切れたら間隔を空けて再接続する。接続できなくても、最後に受け取った方針で制限は続く。
 */
class Link(private val agent: Agent) {
  companion object {
    const val HEARTBEAT_MS = 60_000L
    private const val MAX_BACKOFF_MS = 30_000L
  }

  private val client: OkHttpClient = OkHttpClient.Builder()
    .connectTimeout(10, TimeUnit.SECONDS)
    .pingInterval(HEARTBEAT_MS, TimeUnit.MILLISECONDS)
    .build()

  private val thread = HandlerThread("offsave-link").also { it.start() }
  private val handler = Handler(thread.looper)

  @Volatile private var socket: WebSocket? = null
  @Volatile var connected = false
    private set
  @Volatile var lastError: String? = null
    private set
  @Volatile var lastPolicyAt = 0L
    private set

  private var running = false
  private var backoff = 2_000L
  private var connecting = false
  private val pendingReports = ArrayList<JSONObject>()
  private val sentLabels = HashSet<String>()

  private val tick = object : Runnable {
    override fun run() {
      if (!running) return
      if (connected) sendHeartbeat() else connect()
      handler.postDelayed(this, HEARTBEAT_MS)
    }
  }

  fun start() {
    handler.post {
      if (running) return@post
      running = true
      connect()
      handler.postDelayed(tick, HEARTBEAT_MS)
    }
  }

  /** 止める。`bye` を送るのは、利用者が意図して止めるときだけ。 */
  fun stop(bye: Boolean) {
    handler.post {
      running = false
      handler.removeCallbacksAndMessages(null)
      if (bye) socket?.send(JSONObject().put("type", "bye").toString())
      socket?.close(1000, "bye")
      socket = null
      connected = false
      connecting = false
    }
  }

  private fun wsUrl(base: String): String {
    val lower = base.lowercase()
    val ws = when {
      lower.startsWith("https://") -> "wss://" + base.substring(8)
      lower.startsWith("http://") -> "ws://" + base.substring(7)
      else -> throw IllegalArgumentException("サーバーの URL は http:// か https:// で始めてください")
    }
    return ws.trimEnd('/') + "/api/ws/device"
  }

  private fun connect() {
    if (connecting || connected || !running) return
    val prefs = agent.prefs
    val base = prefs.serverUrl
    val token = prefs.token
    if (base.isNullOrEmpty() || token.isNullOrEmpty()) return
    connecting = true
    val request = try {
      Request.Builder()
        .url(wsUrl(base))
        .header("x-device-token", token)
        .apply { prefs.devUser?.takeIf { it.isNotEmpty() }?.let { header("x-dev-user", it) } }
        .build()
    } catch (e: Exception) {
      lastError = e.message
      connecting = false
      return
    }
    socket = client.newWebSocket(request, object : WebSocketListener() {
      override fun onOpen(webSocket: WebSocket, response: Response) {
        handler.post {
          connecting = false
          connected = true
          lastError = null
          backoff = 2_000L
          sendHeartbeat()
        }
      }

      override fun onMessage(webSocket: WebSocket, text: String) {
        handler.post { handleMessage(text) }
      }

      override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
        handler.post { onLost(webSocket, "閉じられました（$code）") }
      }

      override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
        val why = when (response?.code) {
          401 -> "このデバイスの登録が外れています（削除された可能性があります。設定の「連携を解除する」のあと、もう一度接続してください）"
          null -> t.message ?: t.javaClass.simpleName
          else -> "HTTP ${response.code}"
        }
        handler.post { onLost(webSocket, why) }
      }
    })
  }

  private fun onLost(ws: WebSocket, why: String) {
    if (ws !== socket) return
    connected = false
    connecting = false
    lastError = why
    sentLabels.clear() // 届いたか分からないので、つながったときにもう一度送る
    agent.tracker.onDisconnected()
    if (!running) return
    // 次のハートビートを待たずに、間隔を空けてつなぎ直す
    handler.postDelayed({ connect() }, backoff)
    backoff = minOf(backoff * 2, MAX_BACKOFF_MS)
  }

  // ---- 送信 ----

  /** すぐにハートビートを送る（保護が外れたときなど）。つながっていなければ何もしない。 */
  fun heartbeatNow() {
    handler.post { if (connected) sendHeartbeat() else connect() }
  }

  private fun sendHeartbeat() {
    val ws = socket ?: return
    val now = agent.now()
    val tracker = agent.tracker
    tracker.cut(now)

    val unlocks = tracker.beginUploadUnlocks()
    for (u in unlocks) {
      ws.send(
        JSONObject().put("type", "unlock").put("reason", u.reason)
          .put("started_at", u.startedAt).put("until", u.until).toString(),
      )
    }
    val segments = tracker.beginUploadSegments()
    if (segments.isNotEmpty()) {
      // 初めて送るアプリの表示名を添える（友達の画面でパッケージ名の代わりに出す）
      val fresh = segments.map { it.key }.distinct().filter { it.startsWith("android:") && sentLabels.add(it) }
      val labels = fresh.map { JSONObject().put("key", it).put("label", agent.appLabel(it.removePrefix("android:"))) }
      ws.send(
        JSONObject().put("type", "usage")
          .put("segments", JSONArray(segments.map(Wire::segmentJson)))
          .put("labels", JSONArray(labels)).toString(),
      )
      // アイコンも、初めて送るアプリだけ（友達の画面で、アプリ名の横に出す）
      val icons = fresh.mapNotNull { key ->
        Icons.pngBase64(agent.context, key.removePrefix("android:"))?.let { JSONObject().put("key", key).put("png", it) }
      }
      if (icons.isNotEmpty()) ws.send(JSONObject().put("type", "icons").put("icons", JSONArray(icons)).toString())
    }
    for (r in pendingReports.toList()) ws.send(r.toString())
    pendingReports.clear()

    val flags = agent.flags()
    ws.send(
      JSONObject().put("type", "heartbeat")
        .put(
          "flags",
          JSONObject().put("accessibility", flags.accessibility)
            .put("device_admin", flags.deviceAdmin)
            .put("battery_unrestricted", flags.batteryUnrestricted),
        )
        .put("foreground", tracker.foregroundKey ?: JSONObject.NULL)
        .put("app_version", agent.appVersion)
        .put("last_event_id", tracker.lastEventId ?: JSONObject.NULL)
        .toString(),
    )
    tracker.persist()
  }

  /** 友達に知らせるイベント。つながっていなければ、つながったときに送る。 */
  fun report(kind: String, message: String) {
    val msg = JSONObject().put("type", "report").put("kind", kind).put("message", message)
    handler.post {
      val ws = socket
      if (connected && ws != null) ws.send(msg.toString()) else if (pendingReports.size < 20) pendingReports.add(msg)
    }
  }

  /** 一時解除をすぐ知らせる。つながっていなければ、次のハートビートで送る。 */
  fun unlockNow() = heartbeatNow()

  // ---- 受信 ----

  private fun handleMessage(text: String) {
    val o = try {
      JSONObject(text)
    } catch (_: Exception) {
      return
    }
    when (o.optString("type")) {
      "policy" -> {
        try {
          agent.tracker.applyPolicy(text)
          lastPolicyAt = System.currentTimeMillis()
          agent.onPolicy()
        } catch (e: Exception) {
          lastError = "方針を読めません: ${e.message}"
        }
      }
      "event" -> agent.onEvent(o.getJSONObject("event"))
      "error" -> lastError = o.optString("message")
    }
  }
}
