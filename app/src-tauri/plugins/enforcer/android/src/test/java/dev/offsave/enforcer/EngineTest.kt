package dev.offsave.enforcer

import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Rust と共通の試験データ（crates/core/tests/vectors/engine.json）で、判定の一致を確かめる。 */
class EngineTest {
  private fun vectors(): JSONArray {
    val dir = System.getProperty("vectors.dir") ?: error("vectors.dir が未設定です")
    return JSONArray(File(dir, "engine.json").readText())
  }

  private fun verdictOf(o: JSONObject): Verdict = when (o.getString("type")) {
    "allow" -> Verdict.Allow
    "grace" -> Verdict.Grace(o.getString("rule_id"), o.getLong("until"))
    "block" -> Verdict.Block(
      o.getString("rule_id"),
      if (o.getString("kind") == "window") BlockKind.WINDOW else BlockKind.DAILY_TOTAL,
    )
    else -> error("不明な verdict")
  }

  @Test
  fun engineVectors() {
    val cases = vectors()
    assertTrue(cases.length() >= 10)
    for (i in 0 until cases.length()) {
      val c = cases.getJSONObject(i)
      val name = c.getString("name")
      val got = decide(Wire.input(c.getJSONObject("input")))
      val e = c.getJSONObject("expect")
      assertEquals("verdict: $name", verdictOf(e.getJSONObject("verdict")), got.verdict)
      val graces = e.getJSONArray("new_graces")
      val want = (0 until graces.length()).map { graces.getJSONArray(it).let { p -> p.getString(0) to p.getLong(1) } }
      assertEquals("new_graces: $name", want, got.newGraces)
      val wake = if (e.isNull("next_wakeup")) null else e.getLong("next_wakeup")
      assertEquals("next_wakeup: $name", wake, got.nextWakeup)
    }
  }

  @Test
  fun dateFormatting() {
    assertEquals("1970-01-01", formatDate(0))
    assertEquals("2024-02-29", formatDate(19782))
    assertEquals(3, weekdayOf(0))
  }
}
