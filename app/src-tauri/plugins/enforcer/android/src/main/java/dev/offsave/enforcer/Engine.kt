package dev.offsave.enforcer

/**
 * 制限の判定。crates/core/src/engine.rs の移植。
 * 挙動は crates/core/tests/vectors/engine.json（Rust と共通）で一致を保つ。変えるときは両方を直す。
 */

const val MIN_MS = 60_000L
const val DAY_MS = 86_400_000L
const val GRACE_MS = 60_000L

enum class BlockKind { DAILY_TOTAL, WINDOW }

sealed class Verdict {
  object Allow : Verdict() {
    override fun toString() = "Allow"
  }

  data class Grace(val ruleId: String, val until: Long) : Verdict()
  data class Block(val ruleId: String, val kind: BlockKind) : Verdict()
}

data class Decision(
  val verdict: Verdict,
  val newGraces: List<Pair<String, Long>>,
  val nextWakeup: Long?,
)

data class EngineInput(
  val rules: List<Rule>,
  val overrides: List<Override>,
  val unlockUntil: Long,
  val lastUnlockEnd: Long,
  val segments: List<Segment>,
  val graces: Map<String, Long>,
  val now: Long,
  val clock: Clock,
  val currentKey: String?,
)

// ---- 時刻 ----

class LocalTime(val day: Long, val weekday: Int, val minute: Int)

class Day(val date: String, val weekday: Int, val start: Long, val end: Long)

fun localTime(now: Long, clock: Clock): LocalTime {
  val local = now + clock.utcOffsetMin * MIN_MS
  val day = Math.floorDiv(local, DAY_MS)
  return LocalTime(day, weekdayOf(day), (Math.floorMod(local, DAY_MS) / MIN_MS).toInt())
}

fun logicalDay(now: Long, clock: Clock): Day {
  val shift = (clock.utcOffsetMin - clock.dayBoundaryMin) * MIN_MS
  val d = Math.floorDiv(now + shift, DAY_MS)
  val start = d * DAY_MS - shift
  return Day(formatDate(d), weekdayOf(d), start, start + DAY_MS)
}

/** 0 = 月。1970-01-01 は木曜。 */
fun weekdayOf(day: Long): Int = Math.floorMod(day + 3, 7L).toInt()

/** 日数 → YYYY-MM-DD（Howard Hinnant の civil_from_days）。 */
fun formatDate(days: Long): String {
  val z = days + 719_468
  val era = Math.floorDiv(z, 146_097L)
  val doe = Math.floorMod(z, 146_097L)
  val yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365
  val y = yoe + era * 400
  val doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
  val mp = (5 * doy + 2) / 153
  val d = doy - (153 * mp + 2) / 5 + 1
  val m = if (mp < 10) mp + 3 else mp - 9
  val year = if (m <= 2) y + 1 else y
  return "%04d-%02d-%02d".format(year, m, d)
}

// ---- 区間 ----

/** `[lo, hi)` に切って重ね、和集合の長さ。 */
fun unionLen(ranges: List<Pair<Long, Long>>, lo: Long, hi: Long): Long {
  val clipped = ranges
    .map { (s, e) -> maxOf(s, lo) to minOf(e, hi) }
    .filter { it.first < it.second }
    .sortedBy { it.first }
  var total = 0L
  var curStart = 0L
  var curEnd = 0L
  var open = false
  for ((s, e) in clipped) {
    if (!open) {
      curStart = s; curEnd = e; open = true
    } else if (s <= curEnd) {
      curEnd = maxOf(curEnd, e)
    } else {
      total += curEnd - curStart
      curStart = s; curEnd = e
    }
  }
  if (open) total += curEnd - curStart
  return total
}

fun usedMs(rule: Rule, segments: List<Segment>, dayStart: Long, dayEnd: Long): Long =
  unionLen(segments.filter { rule.matches(it.key) }.map { it.start to it.end }, dayStart, dayEnd)

// ---- 判定 ----

private fun overridden(rule: Rule, overrides: List<Override>, date: String) =
  overrides.any { it.date == date && (it.ruleId == null || it.ruleId == rule.id) }

private fun windowOccurrence(days: Int, start: Int, end: Int, now: Long, clock: Clock): String? {
  val lt = localTime(now, clock)
  fun has(wd: Int) = days and (1 shl wd) != 0
  return if (start < end) {
    if (has(lt.weekday) && lt.minute in start until end) formatDate(lt.day) else null
  } else if (start > end) {
    if (has(lt.weekday) && lt.minute >= start) formatDate(lt.day)
    else if (has((lt.weekday + 6) % 7) && lt.minute < end) formatDate(lt.day - 1)
    else null
  } else {
    null
  }
}

private fun nextWindowStart(days: Int, start: Int, now: Long, clock: Clock): Long? {
  val lt = localTime(now, clock)
  for (d in 0..7) {
    val wd = (lt.weekday + d) % 7
    if (days and (1 shl wd) == 0) continue
    val localStart = (lt.day + d) * DAY_MS + start * MIN_MS
    val utc = localStart - clock.utcOffsetMin * MIN_MS
    if (utc > now) return utc
  }
  return null
}

fun decide(input: EngineInput): Decision {
  val key = input.currentKey ?: return Decision(Verdict.Allow, emptyList(), null)
  val now = input.now
  val day = logicalDay(now, input.clock)

  var wake: Long? = null
  fun wakeAt(t: Long) {
    wake = wake?.let { minOf(it, t) } ?: t
  }

  if (now < input.unlockUntil) {
    return Decision(Verdict.Allow, emptyList(), input.unlockUntil)
  }

  val newGraces = ArrayList<Pair<String, Long>>()
  var block: Verdict.Block? = null
  var grace: Verdict.Grace? = null

  for (rule in input.rules) {
    if (!rule.enabled || !rule.matches(key) || overridden(rule, input.overrides, day.date)) continue

    // 利用時間は、ルールのどの制限でも同じ（必要になったときだけ数える）
    var used: Long? = null
    for ((index, limit) in rule.limits.withIndex()) {
      val occurrence: String?
      val kind: BlockKind
      when (limit) {
        is Limit.DailyTotal -> {
          kind = BlockKind.DAILY_TOTAL
          // 論理日が変わると合計も曜日も変わる
          wakeAt(day.end)
          if (limit.days and (1 shl day.weekday) == 0) {
            occurrence = null
          } else {
            val u = used ?: usedMs(rule, input.segments, day.start, day.end).also { used = it }
            val max = limit.seconds * 1000
            if (u >= max) {
              occurrence = day.date
            } else {
              occurrence = null
              wakeAt(now + (max - u))
            }
          }
        }
        is Limit.Window -> {
          kind = BlockKind.WINDOW
          occurrence = windowOccurrence(limit.days, limit.startMin, limit.endMin, now, input.clock)
          if (occurrence == null) nextWindowStart(limit.days, limit.startMin, now, input.clock)?.let(::wakeAt)
        }
      }
      if (occurrence == null) continue

      val graceKey = "${rule.id}|$index@$occurrence|${input.lastUnlockEnd}"
      val started = input.graces[graceKey] ?: run {
        newGraces.add(graceKey to now)
        now
      }
      val until = started + GRACE_MS
      if (now < until) {
        wakeAt(until)
        if (grace == null || until < grace.until) grace = Verdict.Grace(rule.id, until)
      } else if (block == null) {
        block = Verdict.Block(rule.id, kind)
      }
    }
  }

  val verdict: Verdict = block ?: grace ?: Verdict.Allow
  return Decision(verdict, newGraces, wake)
}
