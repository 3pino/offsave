# offsave

[![CI](https://github.com/3pino/offsave/actions/workflows/ci.yml/badge.svg)](https://github.com/3pino/offsave/actions/workflows/ci.yml)

友達どうしで、お互いのスクリーンタイムを見守るアプリ。Android / Windows / Linux 向け（いまは **Android** が動きます）。

- 友達があなたの制限（アプリ・サイトの使用時間、使えない時間帯）を決める。
- 友達には、使っているアプリ・見ているサイトと、制限の解除・保護の無効化が見える（お互いに）。
- 制限を破るには、理由を書いて、友達に通知が行く「一時解除」をするしかない。
- Tailscale の中だけで動く。インターネットには公開しない。ログインは Tailscale のアカウントで自動（[disnans](https://github.com/3pino/disnans) と同じ仕組み）。

使うには、**全員が合意していること**が前提です。隠して動かす機能はありません（常駐の通知、保護の状態、友達への通知がすべて見える）。

設計の経緯と決定事項は [docs/design/overview.md](docs/design/overview.md)。

## 仕組み

```
Android ── WebSocket（60 秒ごとのハートビート）──▶ サーバー（SQLite）◀── 友達の端末
   │  Accessibility で前面のアプリ・サイトを検知                │  ルール・利用の集計・イベント
   └─ 制限に達したら、猶予 1 分 → 全画面でブロック            └─ ハートビートが途絶えたら友達に通知
```

- 制限の判定は**端末の中**で行う。サーバーやネットワークがなくても、最後に受け取ったルールで制限を続ける。
- 複数のデバイスの利用は、重なった時間を二重に数えず、合計で数える。
- 1 日の境界は、既定で午前 4 時（ユーザーごとに変更でき、変更は友達に通知される）。
- 端末の時計を動かしても、日付や制限は動かない（サーバーの時刻を基準に、経過時間で進める）。

## ビルド済みのファイルを使う

[Actions](https://github.com/3pino/offsave/actions/workflows/ci.yml) の最新の成功した実行の「Artifacts」から取れます（GitHub にログインが必要）。

| ファイル | 内容 |
|---|---|
| `offsave-server-linux-x86_64` | サーバー（Linux x86_64） |
| `offsave-android-arm64-debug` | Android の APK（arm64の実機向け。デバッグ署名） |
| `offsave-desktop-linux` / `offsave-desktop-windows` | デスクトップ版（友達の状況を見るダッシュボード） |

APK のデバッグ署名の鍵は、ビルドのたびに変わります。入れ替えるときは、先にアンインストールが要ります（端末管理者を有効にしている場合は、先に無効にする）。

## サーバーを動かす（管理者）

必要なもの: Rust（stable）、Tailscale

```sh
cargo build --release -p offsave-server
OFFSAVE_BIND=<Tailscale の IP>:8090 OFFSAVE_DATA_DIR=/var/lib/offsave ./target/release/offsave-server
```

| 環境変数 | 既定値 | 内容 |
|---|---|---|
| `OFFSAVE_BIND` | `127.0.0.1:8090` | 待ち受けるアドレス。本番では Tailscale の IP（`tailscale ip -4`） |
| `OFFSAVE_DATA_DIR` | `./data` | DB（`offsave.db`）の置き場所 |
| `OFFSAVE_DEV` | なし | `1` で開発モード（認証をしない。`X-Dev-User` ヘッダーで誰にでもなれる）。**本番では使わない** |
| `OFFSAVE_DEFAULT_TZ` | `Asia/Tokyo` | 新しいユーザーのタイムゾーン |
| `OFFSAVE_SUSPECT_AFTER_SECS` | `180` | ハートビートがこの秒数途絶えたら、友達に「連絡が途絶えた」と通知する |
| `OFFSAVE_TAILSCALE_SOCKET` | 自動 | tailscaled の LocalAPI のソケット |

友達を招待するには、Tailscale の管理画面でサーバー機を[ノード共有](https://tailscale.com/kb/1084/sharing)します（disnans と同じ。ACL でこのポートだけを開ける）。
サーバー管理者は、全員のデータを見られ、DB も書き換えられます。相互監視の前提で、管理者を信頼する運用です。

## Android アプリを入れる

APK は `app/src-tauri/gen/android/app/build/outputs/apk/` にできます（ビルドは下記）。

1. APK を入れて開き、サーバーのアドレス（`http://100.x.y.z:8090`）を入れて「同意して接続する」。
2. ホーム画面の「このデバイス」の 4 項目を、上から順に有効にする。
   - **Accessibility**: 前面のアプリの検知と、制限の表示に使う。Android 13 以降は、APK で入れたアプリの Accessibility に「制限された設定」の許可が要る。ホーム画面の「アプリ情報」→ 右上のメニュー →「制限された設定を許可」。
   - **端末管理者**: 有効にすると、友達に知られずにアンインストールできなくなる。
   - **電池の最適化から除外**: 連絡（ハートビート）が途切れないように。
   - **通知**: 友達の通知を受け取る。
3. 友達タブで、相手の Tailscale のログイン名で申請し、承認してもらう。

## 開発

```sh
cargo test --workspace                       # 判定ロジック・サーバー
cd app && npm install && npm run check       # 画面の型検査
cd app/src-tauri/gen/android && ./gradlew :tauri-plugin-enforcer:testDebugUnitTest   # Android の判定・追跡（Rust と共通の試験データ）
```

Android のビルド（x86_64 のエミュレーター向けのデバッグ版）:

```sh
cd app
export ANDROID_HOME=$HOME/Android/Sdk NDK_HOME=$ANDROID_HOME/ndk/<版> JAVA_HOME=/usr/lib/jvm/java-25-openjdk-amd64
npx tauri android build --debug --target x86_64 --apk       # 実機（arm64）は --target aarch64
```

開発用のサーバー（エミュレーターからは `http://10.0.2.2:8090`）:

```sh
OFFSAVE_DEV=1 OFFSAVE_BIND=127.0.0.1:8090 OFFSAVE_DATA_DIR=/tmp/offsave-dev cargo run -p offsave-server
```

構成:

- `crates/core`: ルールの判定（猶予・一時解除・日付の境界）、利用区間の和集合、サーバーとの型。
- `crates/server`: axum + SQLite。REST と、デバイスの WebSocket（`/api/ws/device`）。
- `app`: Tauri + Svelte の画面。
- `app/src-tauri/plugins/enforcer`: Android の強制（Kotlin）。`Engine.kt` は `crates/core` の判定の移植で、`crates/core/tests/vectors/engine.json` を両方で試験して一致を保つ。

## 既知の限界

検知も通知もできないもの（[設計メモ](docs/design/overview.md#7-既知の限界検知も通知もできないもの)）:

- Android: 別プロファイル（Work profile・Private Space・アプリのクローン）の中での利用内容。
- 全 OS: 未登録のデバイスでの利用。
- ブラウザは、URL バーを読める対応ブラウザだけで、サイトを数える。対応外のブラウザを使うと、友達に通知される。
- 対応ブラウザのうち、実際に確認したのは Chrome だけ。ほかは同じ系統の ID で書いてあるが、実機での確認が要る。
