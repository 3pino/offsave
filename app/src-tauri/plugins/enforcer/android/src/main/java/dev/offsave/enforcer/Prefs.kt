package dev.offsave.enforcer

import android.content.Context

/** サーバーへの接続情報など、このデバイスの設定。 */
class Prefs(context: Context) {
  private val sp = context.applicationContext.getSharedPreferences("offsave", Context.MODE_PRIVATE)

  var serverUrl: String?
    get() = sp.getString("server_url", null)
    set(v) = sp.edit().putString("server_url", v).apply()

  /** 開発用: サーバーを開発モードで動かしているときの、なりすますユーザー名。本番では空。 */
  var devUser: String?
    get() = sp.getString("dev_user", null)
    set(v) = sp.edit().putString("dev_user", v).apply()

  var deviceId: String?
    get() = sp.getString("device_id", null)
    set(v) = sp.edit().putString("device_id", v).apply()

  var token: String?
    get() = sp.getString("token", null)
    set(v) = sp.edit().putString("token", v).apply()

  var userId: String?
    get() = sp.getString("user_id", null)
    set(v) = sp.edit().putString("user_id", v).apply()

  /** 設定画面での無効化・アンインストールの操作を検知して、ホームに戻す。 */
  var guardSettings: Boolean
    get() = sp.getBoolean("guard_settings", true)
    set(v) = sp.edit().putBoolean("guard_settings", v).apply()

  /** この時刻まで、設定画面の見張りを止める（権限の設定の案内で、アプリ自身が設定画面を開くとき）。 */
  var setupUntil: Long
    get() = sp.getLong("setup_until", 0)
    set(v) = sp.edit().putLong("setup_until", v).apply()

  /** 見張っている Android の設定の、前回の状態（JSON）。`SettingsWatch` が使う。 */
  var watchedState: String?
    get() = sp.getString("watched_state", null)
    set(v) = sp.edit().putString("watched_state", v).apply()

  /** 一時停止中（Accessibility を自分で外している）の決済アプリのパッケージ名。していなければ null。 */
  var pausePackage: String?
    get() = sp.getString("pause_package", null)
    set(v) = sp.edit().putString("pause_package", v).apply()

  /** 一時停止を始めた時刻（端末の時計。UsageStats の出来事と同じ基準）。 */
  var pauseStartedAt: Long
    get() = sp.getLong("pause_started_at", 0)
    set(v) = sp.edit().putLong("pause_started_at", v).apply()

  val configured: Boolean get() = !serverUrl.isNullOrEmpty() && !token.isNullOrEmpty()

  fun clear() {
    sp.edit().clear().apply()
  }
}

/** [Tracker] の状態の保存先。 */
class PrefsStore(context: Context) : StateStore {
  private val sp = context.applicationContext.getSharedPreferences("offsave_state", Context.MODE_PRIVATE)
  override fun load(): String? = sp.getString("state", null)
  override fun save(json: String) {
    sp.edit().putString("state", json).apply()
  }

  fun clear() {
    sp.edit().clear().apply()
  }
}
