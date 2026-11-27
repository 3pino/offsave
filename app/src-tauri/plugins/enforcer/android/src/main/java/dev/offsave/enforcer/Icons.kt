package dev.offsave.enforcer

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.util.Base64
import java.io.ByteArrayOutputStream

/**
 * アプリのアイコンを、小さな PNG（base64）にする。友達の画面で、アプリ名の横に出すのに使う。
 * サーバーが受け取る上限（16KB）に収まるまで、64 → 48 → 32 px と小さくする。
 */
object Icons {
  private val SIZES = intArrayOf(64, 48, 32)
  private const val MAX_BYTES = 15 * 1024

  fun pngBase64(context: Context, pkg: String): String? = try {
    val drawable = context.packageManager.getApplicationIcon(pkg)
    SIZES.asList().firstNotNullOfOrNull { px ->
      val bmp = Bitmap.createBitmap(px, px, Bitmap.Config.ARGB_8888)
      try {
        drawable.setBounds(0, 0, px, px)
        drawable.draw(Canvas(bmp))
        val out = ByteArrayOutputStream()
        bmp.compress(Bitmap.CompressFormat.PNG, 100, out)
        out.toByteArray().takeIf { it.size <= MAX_BYTES }
      } finally {
        bmp.recycle()
      }
    }?.let { Base64.encodeToString(it, Base64.NO_WRAP) }
  } catch (_: Exception) {
    null
  }
}
