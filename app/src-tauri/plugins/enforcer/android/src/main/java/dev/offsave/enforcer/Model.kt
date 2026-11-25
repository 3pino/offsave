package dev.offsave.enforcer

import org.json.JSONArray
import org.json.JSONObject

/** サーバーとの型（crates/core の model・protocol と同じ JSON）。 */

data class Segment(val key: String, val start: Long, val end: Long)

data class Target(val label: String, val keys: List<String>)

sealed class Limit {
  /** 使えない時間帯。days は開始する曜日のビット（bit0 = 月）。 */
  data class Window(val days: Int, val startMin: Int, val endMin: Int) : Limit()
  data class DailyTotal(val seconds: Long) : Limit()
}

data class Rule(
  val id: String,
  val name: String,
  val targets: List<Target>,
  val limit: Limit,
  val enabled: Boolean,
) {
  fun matches(key: String): Boolean = targets.any { t -> t.keys.any { keyMatches(it, key) } }
}

data class Override(val id: String, val date: String, val ruleId: String?)

data class Clock(val utcOffsetMin: Int, val dayBoundaryMin: Int) {
  companion object {
    val DEFAULT = Clock(540, 240)
  }
}

data class Policy(
  val serverTime: Long,
  val clock: Clock,
  val rules: List<Rule>,
  val overrides: List<Override>,
  val unlockUntil: Long,
  val lastUnlockEnd: Long,
  val unlockMinutes: Int,
  val others: List<Segment>,
)

/** 識別子が対象に当たるか。`domain:` はサブドメインも含む。 */
fun keyMatches(target: String, key: String): Boolean {
  if (target == key) return true
  if (target.startsWith("domain:") && key.startsWith("domain:")) {
    return key.removePrefix("domain:").endsWith("." + target.removePrefix("domain:"))
  }
  return false
}

/** JSON の読み書き。 */
object Wire {
  private fun JSONObject.optStringOrNull(name: String): String? =
    if (isNull(name)) null else getString(name)

  fun <T> JSONArray.map(f: (JSONObject) -> T): List<T> =
    (0 until length()).map { f(getJSONObject(it)) }

  fun segment(o: JSONObject) = Segment(o.getString("key"), o.getLong("start"), o.getLong("end"))

  fun segmentJson(s: Segment): JSONObject =
    JSONObject().put("key", s.key).put("start", s.start).put("end", s.end)

  fun clock(o: JSONObject) = Clock(o.getInt("utc_offset_min"), o.getInt("day_boundary_min"))

  fun rule(o: JSONObject): Rule {
    val l = o.getJSONObject("limit")
    val limit = when (l.getString("type")) {
      "window" -> Limit.Window(l.getInt("days"), l.getInt("start_min"), l.getInt("end_min"))
      else -> Limit.DailyTotal(l.getLong("seconds"))
    }
    return Rule(
      id = o.getString("id"),
      name = o.getString("name"),
      targets = o.getJSONArray("targets").map { t ->
        Target(t.getString("label"), (0 until t.getJSONArray("keys").length()).map { t.getJSONArray("keys").getString(it) })
      },
      limit = limit,
      enabled = o.getBoolean("enabled"),
    )
  }

  fun override(o: JSONObject) = Override(o.getString("id"), o.getString("date"), o.optStringOrNull("rule_id"))

  fun policy(o: JSONObject) = Policy(
    serverTime = o.getLong("server_time"),
    clock = clock(o.getJSONObject("clock")),
    rules = o.getJSONArray("rules").map(::rule),
    overrides = o.getJSONArray("overrides").map(::override),
    unlockUntil = o.getLong("unlock_until"),
    lastUnlockEnd = o.getLong("last_unlock_end"),
    unlockMinutes = o.getInt("unlock_minutes"),
    others = o.getJSONArray("others").map(::segment),
  )

  fun input(o: JSONObject): EngineInput {
    val graces = HashMap<String, Long>()
    o.optJSONObject("graces")?.let { g -> g.keys().forEach { graces[it] = g.getLong(it) } }
    return EngineInput(
      rules = o.getJSONArray("rules").map(::rule),
      overrides = o.getJSONArray("overrides").map(::override),
      unlockUntil = o.getLong("unlock_until"),
      lastUnlockEnd = o.getLong("last_unlock_end"),
      segments = o.getJSONArray("segments").map(::segment),
      graces = graces,
      now = o.getLong("now"),
      clock = clock(o.getJSONObject("clock")),
      currentKey = o.optStringOrNull("current_key"),
    )
  }
}
