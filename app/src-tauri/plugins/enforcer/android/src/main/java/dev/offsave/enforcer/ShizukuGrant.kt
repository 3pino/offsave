package dev.offsave.enforcer

import android.content.Context
import android.content.pm.PackageManager
import rikka.shizuku.Shizuku

/**
 * Shizuku が動いていれば、「設定の書き込みの権限」（WRITE_SECURE_SETTINGS）を、PC なしで付ける。
 * Shizuku は付ける 1 回だけに使う（付けたあとは、Shizuku が止まっても権限は残る）。
 */
object ShizukuGrant {
  enum class Result(val code: String) {
    OK("ok"),

    /** Shizuku が入っていない・動いていない。 */
    NOT_RUNNING("not_running"),

    /** Shizuku のバージョンが古い。 */
    TOO_OLD("too_old"),

    /** Shizuku の画面で、許可されなかった。 */
    DENIED("denied"),

    /** コマンドが失敗した。 */
    FAILED("failed"),
  }

  private const val REQUEST_CODE = 7101
  private const val PERMISSION = "android.permission.WRITE_SECURE_SETTINGS"

  /** Shizuku が動いていて、使える状態か。 */
  fun running(): Boolean = try {
    Shizuku.pingBinder()
  } catch (_: Throwable) {
    false
  }

  /** 付ける。Shizuku の許可の画面が出ることがある。結果は `done` に返る（どのスレッドでも）。 */
  fun grant(context: Context, done: (Result) -> Unit) {
    try {
      if (!running()) return done(Result.NOT_RUNNING)
      if (Shizuku.isPreV11()) return done(Result.TOO_OLD)
      if (Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED) {
        return runGrant(context, done)
      }
      // 「今後は表示しない」で拒否されていると、画面が出ないので、Shizuku 側で許可してもらう
      if (Shizuku.shouldShowRequestPermissionRationale()) return done(Result.DENIED)
      lateinit var listener: Shizuku.OnRequestPermissionResultListener
      listener = Shizuku.OnRequestPermissionResultListener { code, result ->
        if (code != REQUEST_CODE) return@OnRequestPermissionResultListener
        Shizuku.removeRequestPermissionResultListener(listener)
        if (result == PackageManager.PERMISSION_GRANTED) runGrant(context, done) else done(Result.DENIED)
      }
      Shizuku.addRequestPermissionResultListener(listener)
      Shizuku.requestPermission(REQUEST_CODE)
    } catch (_: Throwable) {
      done(Result.NOT_RUNNING)
    }
  }

  private fun runGrant(context: Context, done: (Result) -> Unit) {
    Thread {
      val result = try {
        // newProcess は非公開（Shizuku の API が UserService を勧めているため）だが、`pm grant` を 1 回打つだけなので、これで足りる
        val m = Shizuku::class.java.getDeclaredMethod(
          "newProcess", Array<String>::class.java, Array<String>::class.java, String::class.java,
        )
        m.isAccessible = true
        val process = m.invoke(null, arrayOf("pm", "grant", context.packageName, PERMISSION), null, null) as Process
        if (process.waitFor() == 0) Result.OK else Result.FAILED
      } catch (_: Throwable) {
        Result.FAILED
      }
      done(result)
    }.start()
  }
}
