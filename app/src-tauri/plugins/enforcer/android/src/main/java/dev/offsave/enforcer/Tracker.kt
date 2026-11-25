package dev.offsave.enforcer

import org.json.JSONArray
import org.json.JSONObject

/** 状態の保存先（Android では SharedPreferences、試験ではメモリ）。 */
interface StateStore {
  fun load(): String?
  fun save(json: String)
}

class MemoryStore : StateStore {
  var value: String? = null
  override fun load() = value
  override fun save(json: String) {
    value = json
  }
}

data class PendingUnlock(val reason: String, val startedAt: Long, val until: Long)

/**
 * 前面のアプリの利用区間を記録し、制限を判定する。Android に依存しない（試験できる）。
 *
 * - 区間はこのデバイスの分だけを持つ。ほかのデバイスの分は、サーバーの方針（`others`）で受け取る。
 * - アップロードは「送ってから、次の方針の返事が来たら届いたとみなす」。サーバーは同じ区間の再送を受け付ける。
 * - 状態は保存し、再起動やオフラインのあとも、猶予・一時解除・未送信の区間を失わない。
 */
class Tracker(private val store: StateStore) {
  private class Open(val key: String, val start: Long)

  private var policyJson: String? = null
  var policy: Policy? = null
    private set
  private val segments = ArrayList<Segment>()
  private val pending = ArrayList<Segment>()
  private val graces = HashMap<String, Long>()
  private var localUnlockMax = 0L
  private val pendingUnlocks = ArrayList<PendingUnlock>()
  private var open: Open? = null
  private var inflightSegments = 0
  private var inflightUnlocks = 0
  var lastEventId: String? = null
    private set

  /** いま前面のもの（画面オフ・不明は null）。 */
  val currentKey: String? get() = open?.key

  init {
    load()
  }

  private fun load() {
    val text = store.load() ?: return
    try {
      val o = JSONObject(text)
      o.optString("policy", "").takeIf { it.isNotEmpty() }?.let {
        policyJson = it
        policy = Wire.policy(JSONObject(it))
      }
      with(Wire) {
        o.optJSONArray("segments")?.map(::segment)?.let { segments.addAll(it) }
        o.optJSONArray("pending")?.map(::segment)?.let { pending.addAll(it) }
        o.optJSONArray("pending_unlocks")?.map {
          PendingUnlock(it.getString("reason"), it.getLong("started_at"), it.getLong("until"))
        }?.let { pendingUnlocks.addAll(it) }
      }
      o.optJSONObject("graces")?.let { g -> g.keys().forEach { graces[it] = g.getLong(it) } }
      localUnlockMax = o.optLong("local_unlock_max", 0)
      lastEventId = o.optString("last_event_id", "").takeIf { it.isNotEmpty() }
    } catch (_: Exception) {
      // 壊れた保存は捨てる（制限は次の方針で戻る）
    }
  }

  @Synchronized
  fun persist() {
    val o = JSONObject()
    policyJson?.let { o.put("policy", it) }
    o.put("segments", JSONArray(segments.map(Wire::segmentJson)))
    o.put("pending", JSONArray(pending.map(Wire::segmentJson)))
    o.put("graces", JSONObject(graces as Map<*, *>))
    o.put("local_unlock_max", localUnlockMax)
    o.put(
      "pending_unlocks",
      JSONArray(pendingUnlocks.map {
        JSONObject().put("reason", it.reason).put("started_at", it.startedAt).put("until", it.until)
      }),
    )
    lastEventId?.let { o.put("last_event_id", it) }
    store.save(o.toString())
  }

  // ---- 方針 ----

  @Synchronized
  fun applyPolicy(json: String) {
    val p = Wire.policy(JSONObject(json))
    policyJson = json
    policy = p
    // 古い猶予の記録は捨てる（キーの先頭の日付が 3 日より前）
    val horizon = p.serverTime - 3 * DAY_MS
    graces.entries.removeAll { it.value < horizon }
    // サーバーに届いたものは消す
    ackUpload()
    val dayStart = logicalDay(p.serverTime, p.clock).start - DAY_MS
    segments.removeAll { it.end < dayStart }
    persist()
  }

  @Synchronized
  fun noteEvent(id: String) {
    lastEventId = id
  }

  // ---- 前面のアプリ ----

  /** 前面が変わった。`key` は `android:<package>`、画面オフなどは null。 */
  @Synchronized
  fun onForeground(key: String?, now: Long) {
    if (open?.key == key) return
    closeOpen(now)
    open = key?.let { Open(it, now) }
  }

  private fun closeOpen(now: Long) {
    val o = open ?: return
    if (now > o.start) addSegment(Segment(o.key, o.start, now))
  }

  private fun addSegment(s: Segment) {
    segments.add(s)
    pending.add(s)
  }

  /** 続いている区間をここで区切って、未送信の一覧に入れる（ハートビートのたびに呼ぶ）。 */
  @Synchronized
  fun cut(now: Long) {
    val o = open ?: return
    if (now > o.start) {
      addSegment(Segment(o.key, o.start, now))
      open = Open(o.key, now)
    }
  }

  // ---- アップロード ----

  /** 送る区間。送ったら、返事が来るまで消さない。 */
  @Synchronized
  fun beginUploadSegments(): List<Segment> {
    inflightSegments = pending.size
    return pending.toList()
  }

  @Synchronized
  fun beginUploadUnlocks(): List<PendingUnlock> {
    inflightUnlocks = pendingUnlocks.size
    return pendingUnlocks.toList()
  }

  /** 方針の返事が来た = それまでに送ったものは届いた。 */
  @Synchronized
  fun ackUpload() {
    repeat(minOf(inflightSegments, pending.size)) { pending.removeAt(0) }
    repeat(minOf(inflightUnlocks, pendingUnlocks.size)) { pendingUnlocks.removeAt(0) }
    inflightSegments = 0
    inflightUnlocks = 0
  }

  /** 切断した。送ったものが届いたかは分からないので、次の接続でもう一度送る。 */
  @Synchronized
  fun onDisconnected() {
    inflightSegments = 0
    inflightUnlocks = 0
  }

  // ---- 一時解除 ----

  @Synchronized
  fun unlockUntil(now: Long): Long {
    val server = policy?.unlockUntil ?: 0
    val local = if (localUnlockMax > now) localUnlockMax else 0
    return maxOf(server, local).let { if (it > now) it else 0 }
  }

  /** 一時解除する。終わりの時刻を返す。 */
  @Synchronized
  fun startUnlock(reason: String, now: Long): Long {
    val minutes = policy?.unlockMinutes ?: 15
    val until = now + minutes * MIN_MS
    localUnlockMax = maxOf(localUnlockMax, until)
    pendingUnlocks.add(PendingUnlock(reason, now, until))
    persist()
    return until
  }

  // ---- 判定 ----

  /** 今の状態を判定する。新しい猶予は記録する。 */
  @Synchronized
  fun decide(now: Long): Decision {
    val p = policy ?: return Decision(Verdict.Allow, emptyList(), null)
    val all = ArrayList<Segment>(p.others.size + segments.size + 1)
    all.addAll(p.others)
    all.addAll(segments)
    open?.let { all.add(Segment(it.key, it.start, now)) }
    val lastEnd = maxOf(p.lastUnlockEnd, if (localUnlockMax <= now) localUnlockMax else 0)
    val unlock = unlockUntil(now)
    val d = decide(
      EngineInput(
        rules = p.rules,
        overrides = p.overrides,
        unlockUntil = unlock,
        lastUnlockEnd = lastEnd,
        segments = all,
        graces = graces,
        now = now,
        clock = p.clock,
        currentKey = open?.key,
      ),
    )
    if (d.newGraces.isNotEmpty()) {
      d.newGraces.forEach { graces[it.first] = it.second }
      persist()
    }
    return d
  }

  /** ルール名（オーバーレイの表示用）。 */
  fun ruleName(id: String): String? = policy?.rules?.firstOrNull { it.id == id }?.name
}
