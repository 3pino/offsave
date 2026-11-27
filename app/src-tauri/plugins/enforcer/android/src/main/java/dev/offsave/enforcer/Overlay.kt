package dev.offsave.enforcer

import android.accessibilityservice.AccessibilityService
import android.content.Context
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.text.InputType
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/**
 * 制限の表示。Accessibility Service のオーバーレイ窓（追加の権限が要らない）に出す。
 *
 * - 猶予: 画面の上に小さな帯（あと何秒か）。操作は邪魔しない。
 * - ブロック: 全画面。「ホームに戻る」と、一時解除の手順（理由の入力 → 確認 → 10 秒待ってから決定）。
 * main スレッドで使う。
 */
class Overlay(private val service: AccessibilityService, private val agent: Agent) {
  private val wm = service.getSystemService(Context.WINDOW_SERVICE) as WindowManager
  private val main = Handler(Looper.getMainLooper())
  private var banner: TextView? = null
  private var block: BlockView? = null

  val visible: Boolean get() = block != null || banner != null

  private var lastChangeAt = 0L

  /** 出し入れしてから、まだ間もないか。 */
  fun changedRecently(): Boolean = SystemClock.elapsedRealtime() - lastChangeAt < 700

  fun apply(verdict: Verdict, ruleName: String?) {
    when (verdict) {
      is Verdict.Allow -> hideAll()
      is Verdict.Grace -> {
        hideBlock()
        showBanner(verdict.until, ruleName ?: "制限")
      }
      is Verdict.Block -> {
        hideBanner()
        val name = ruleName ?: "制限"
        showBlock(
          when (verdict.kind) {
            BlockKind.WINDOW -> "「$name」の使えない時間帯です。"
            BlockKind.DAILY_TOTAL -> "「$name」の 1 日の上限に達しました。"
          },
        )
      }
    }
  }

  fun hideAll() {
    hideBanner()
    hideBlock()
  }

  // ---- 猶予の帯 ----

  private var bannerUntil = 0L
  private var bannerRule = ""
  private val bannerTick = object : Runnable {
    override fun run() {
      val b = banner ?: return
      val left = ((bannerUntil - agent.now() + 999) / 1000).coerceAtLeast(0)
      b.text = "「$bannerRule」の制限に達しました。あと ${left} 秒で使えなくなります"
      main.postDelayed(this, 1000)
    }
  }

  private fun showBanner(until: Long, rule: String) {
    bannerUntil = until
    bannerRule = rule
    if (banner == null) {
      lastChangeAt = SystemClock.elapsedRealtime()
      val tv = TextView(service).apply {
        setTextColor(Color.WHITE)
        textSize = 15f
        gravity = Gravity.CENTER
        setPadding(dp(16), dp(10), dp(16), dp(10))
        background = GradientDrawable().apply {
          setColor(0xEED32F2F.toInt())
          cornerRadius = dp(12).toFloat()
        }
      }
      val lp = WindowManager.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
        WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
        WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
          WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or
          WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
        PixelFormat.TRANSLUCENT,
      ).apply {
        gravity = Gravity.TOP
        y = dp(40)
        x = 0
      }
      wm.addView(tv, lp)
      banner = tv
    }
    main.removeCallbacks(bannerTick)
    bannerTick.run()
  }

  private fun hideBanner() {
    main.removeCallbacks(bannerTick)
    banner?.let {
      lastChangeAt = SystemClock.elapsedRealtime()
      runCatching { wm.removeView(it) }
    }
    banner = null
  }

  // ---- ブロックの全画面 ----

  /** `rule` は、理由の文（「「ゲーム」の 1 日の上限に達しました。」など）。 */
  private fun showBlock(rule: String) {
    block?.let {
      it.setRule(rule)
      return
    }
    lastChangeAt = SystemClock.elapsedRealtime()
    val v = BlockView(service, agent, rule)
    val lp = WindowManager.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      ViewGroup.LayoutParams.MATCH_PARENT,
      WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
      // 文字の入力のために、フォーカスを受ける（FLAG_NOT_FOCUSABLE を付けない）
      WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
      PixelFormat.TRANSLUCENT,
    ).apply {
      softInputMode = WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE
    }
    wm.addView(v, lp)
    block = v
  }

  private fun hideBlock() {
    block?.let {
      lastChangeAt = SystemClock.elapsedRealtime()
      it.dispose()
      runCatching { wm.removeView(it) }
    }
    block = null
  }

  private fun dp(v: Int) = (v * service.resources.displayMetrics.density).toInt()
}

private fun Context.dp(v: Int) = (v * resources.displayMetrics.density).toInt()

/** ブロックの画面。3 つの段階がある: 案内 → 理由の入力 → 確認（10 秒待つ）。 */
private class BlockView(
  context: Context,
  private val agent: Agent,
  private var rule: String,
) : FrameLayout(context) {
  private val main = Handler(Looper.getMainLooper())
  private val content = LinearLayout(context).apply {
    orientation = LinearLayout.VERTICAL
    setPadding(context.dp(24), context.dp(48), context.dp(24), context.dp(24))
  }
  private var reason = ""
  private var countdown: Runnable? = null

  companion object {
    private const val CONFIRM_WAIT_SEC = 10
    private const val MIN_REASON_CHARS = 3
  }

  init {
    setBackgroundColor(0xFF101418.toInt())
    isFocusableInTouchMode = true
    addView(
      ScrollView(context).apply {
        isFillViewport = true
        addView(content)
      },
      LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT),
    )
    showIntro()
  }

  fun setRule(name: String) {
    if (name != rule) {
      rule = name
      showIntro()
    }
  }

  fun dispose() {
    countdown?.let { main.removeCallbacks(it) }
  }

  /** 戻るキーは、ホームに戻る（この画面の裏のアプリには戻さない）。 */
  override fun dispatchKeyEvent(event: KeyEvent): Boolean {
    if (event.keyCode == KeyEvent.KEYCODE_BACK) {
      if (event.action == KeyEvent.ACTION_UP) agent.goHome()
      return true
    }
    return super.dispatchKeyEvent(event)
  }

  // ---- 部品 ----

  private fun title(text: String) = TextView(context).apply {
    this.text = text
    setTextColor(Color.WHITE)
    textSize = 26f
    setPadding(0, 0, 0, context.dp(12))
  }

  private fun body(text: String) = TextView(context).apply {
    this.text = text
    setTextColor(0xFFDDDDDD.toInt())
    textSize = 16f
    setLineSpacing(0f, 1.2f)
    setPadding(0, 0, 0, context.dp(20))
  }

  private fun button(text: String, primary: Boolean, onClick: () -> Unit) = Button(context).apply {
    this.text = text
    isAllCaps = false
    setTextColor(Color.WHITE)
    textSize = 16f
    background = GradientDrawable().apply {
      setColor(if (primary) 0xFF1F6FEB.toInt() else 0xFF2D333B.toInt())
      cornerRadius = context.dp(12).toFloat()
    }
    setOnClickListener { onClick() }
    layoutParams = LinearLayout.LayoutParams(
      ViewGroup.LayoutParams.MATCH_PARENT,
      context.dp(52),
    ).apply { bottomMargin = context.dp(12) }
  }

  private fun reset(vararg views: View) {
    countdown?.let { main.removeCallbacks(it) }
    countdown = null
    // 入力欄が無くなる段階では、キーボードを閉じる
    (context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager)
      .hideSoftInputFromWindow(windowToken, 0)
    content.removeAllViews()
    views.forEach { content.addView(it) }
  }

  // ---- 段階 ----

  private fun showIntro() {
    reset(
      title("利用制限中"),
      body("${rule}このアプリは今は使えません。"),
      button("ホームに戻る", primary = true) { agent.goHome() },
      button("制限を一時的に解除する…", primary = false) { showReason() },
    )
  }

  private fun showReason() {
    val input = EditText(context).apply {
      hint = "理由を入力（${MIN_REASON_CHARS}文字以上）"
      setHintTextColor(0xFF8B949E.toInt())
      setTextColor(Color.WHITE)
      textSize = 16f
      inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
      minLines = 3
      gravity = Gravity.TOP or Gravity.START
      setPadding(context.dp(12), context.dp(12), context.dp(12), context.dp(12))
      background = GradientDrawable().apply {
        setColor(0xFF1C2128.toInt())
        cornerRadius = context.dp(8).toFloat()
      }
      setText(reason)
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
      ).apply { bottomMargin = context.dp(16) }
    }
    val error = TextView(context).apply {
      setTextColor(0xFFFF7B72.toInt())
      textSize = 14f
      visibility = View.GONE
    }
    reset(
      title("制限を一時的に解除する"),
      body("なぜ制限を解除する必要があるのかを書いてください。この文章は、そのまま友達への通知の本文になります。"),
      input,
      error,
      button("次へ", primary = true) {
        val text = input.text.toString().trim()
        if (text.codePointCount(0, text.length) < MIN_REASON_CHARS) {
          error.text = "理由を${MIN_REASON_CHARS}文字以上で書いてください"
          error.visibility = View.VISIBLE
        } else {
          reason = text
          showConfirm()
        }
      },
      button("戻る", primary = false) {
        reason = input.text.toString()
        showIntro()
      },
    )
    input.requestFocus()
  }

  private fun showConfirm() {
    val quote = TextView(context).apply {
      text = reason
      setTextColor(Color.WHITE)
      textSize = 16f
      setPadding(context.dp(12), context.dp(12), context.dp(12), context.dp(12))
      background = GradientDrawable().apply {
        setColor(0xFF1C2128.toInt())
        cornerRadius = context.dp(8).toFloat()
      }
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT,
      ).apply { bottomMargin = context.dp(20) }
    }
    val minutes = agent.tracker.policy?.unlockMinutes ?: 15
    val confirm = button("あと ${CONFIRM_WAIT_SEC} 秒お待ちください", primary = true) {
      agent.unlock(reason)
      reason = ""
    }.apply { isEnabled = false; alpha = 0.5f }

    reset(
      title("本当に必要ですか？"),
      body("この制限の解除は、本当にあなたに必要ですか？\n友達に次の内容が通知されますが、よいですか？"),
      quote,
      confirm,
      button("やめる", primary = false) { showIntro() },
    )

    var left = CONFIRM_WAIT_SEC
    val tick = object : Runnable {
      override fun run() {
        left--
        if (left <= 0) {
          confirm.text = "解除する（${minutes}分）"
          confirm.isEnabled = true
          confirm.alpha = 1f
        } else {
          confirm.text = "あと $left 秒お待ちください"
          main.postDelayed(this, 1000)
        }
      }
    }
    countdown = tick
    main.postDelayed(tick, 1000)
  }
}
