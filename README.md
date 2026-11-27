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

[Releases](https://github.com/3pino/offsave/releases/latest) に、署名済みのファイルがあります。

| ファイル | 内容 |
|---|---|
| `offsave_<版>_android-arm64.apk` | Android（arm64 の実機） |
| `offsave_<版>_x64-setup.exe` | Windows（友達の状況を見るダッシュボード） |
| `offsave_<版>_amd64.AppImage` / `.deb` | Linux（同上） |
| `offsave-server_linux-x64`（`.sha256`） | サーバー（Linux x86_64） |

## 更新の仕組み

- **Android**: アプリの「設定」→「アプリの更新」が、GitHub の最新リリースを見て、APK を取ってインストーラーを開く。新しい版があると、トーストと「設定」の点で知らせる。APK はリリースの鍵で署名してあり、Android のインストーラーが「入っているアプリと同じ鍵」でないと入れない（ダウンロード元もこのリポジトリのリリースに限る）。
- **Windows / Linux（デスクトップ）**: Tauri のアップデーター。`latest.json` を見て、署名（minisign）を確かめてから入れる。公開鍵はアプリに入っている。
- **サーバー**: `OFFSAVE_AUTO_UPDATE=1` で、最新の正式版を 10 分おきに見て、sha256 と `--version` を確かめ、DB をバックアップしてから入れ替えて再起動する。

### リリースの作り方

```sh
scripts/bump-version.sh 0.3.0        # 版をそろえる（Tauri の設定・アプリ・サーバー）
git commit -am "v0.3.0" && git push
git tag -a v0.3.0 -m "..." && git push origin v0.3.0   # Actions がビルドし、署名して、Release を公開する
```

署名の鍵（手元の `~/.config/offsave-keys/`、GitHub の Secrets にも同じもの）:

| 鍵 | 用途 | 失うと |
|---|---|---|
| `android.jks`（`android.password`） | APK の署名 | 既存のアプリを更新できない（全員がアンインストールして入れ直し。デバイス管理者の解除も必要） |
| `updater.key`（`updater.password`） | デスクトップの自動更新の署名 | 既存のデスクトップ版が自動更新できない（入れ直し） |

**鍵は必ずバックアップすること**（リポジトリには入れない）。手元でリリース署名の APK を作るときも、同じ鍵を `~/.config/offsave-keys/` から読む。

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
| `OFFSAVE_SUSPECT_AFTER_SECS` | `600` | ハートビートがこの秒数途絶えたら、友達に「連絡が途絶えた」と通知する |
| `OFFSAVE_TAILSCALE_SOCKET` | 自動 | tailscaled の LocalAPI のソケット |
| `OFFSAVE_AUTO_UPDATE` | なし | `1` で自動更新（上の「更新の仕組み」）。systemd の `Restart=on-failure` が、入れ替えたあとの再起動をする |

友達を招待するには、Tailscale の管理画面でサーバー機を[ノード共有](https://tailscale.com/kb/1084/sharing)します（disnans と同じ）。**共有で入った人は、ACL で許可したポートにしか届きません。** disnans を共有済みでも、offsave のポートは別に足す必要があります（足さないと、友達の側は接続が止まったままになります）。

```jsonc
// Access Controls（login.tailscale.com/admin/acls）の "acls" に足す。"server" は hosts に書いたサーバー機の名前
{ "action": "accept", "src": ["autogroup:shared"], "dst": ["server:8090"] }
```
サーバー管理者は、全員のデータを見られ、DB も書き換えられます。相互監視の前提で、管理者を信頼する運用です。

## Android アプリを入れる

リリースの APK を入れます（以後は、アプリの「設定」から更新できます）。

1. APK を入れて開き、サーバーのアドレス（`http://100.x.y.z:8090`）を入れて「同意して接続する」。
2. 「設定」→「このデバイスの保護」の 4 項目を、上から順に有効にする（足りない間は、ホームに警告と、設定のナビに点が出る）。
   - **Accessibility**: 前面のアプリの検知と、制限の表示に使う。Android 13 以降は、APK で入れたアプリの Accessibility に「制限された設定」の許可が要る。「アプリ情報」→ 右上のメニュー →「制限された設定を許可」。
   - **デバイス管理者**: 有効にすると、友達に知られずにアンインストールできなくなる。
   - **電池の最適化から除外**: 連絡（ハートビート）が途切れないように。
   - **通知**: 友達の通知を受け取る。
3. 友達タブで、相手を検索して申請し、承認してもらう。

### 決済アプリ（PayPay など）を使うとき

PayPay など一部のアプリは、Accessibility が有効なデバイスでは使えません（「ユーザー補助の設定を変更してください」と表示されて、offsave を OFF にするよう求められます）。権限や宣言を変えても避けられないことを確かめてあります。

そこで、**友達が決めたアプリが前面にある間だけ、offsave が自分で Accessibility を外し、離れたら戻します**。外すのと戻すのは、友達に通知されます。

- 対象のアプリは、友達が「ルール」タブの「保護を一時停止するアプリ」で決めます。本人は追加できません。
- 外している間も、利用時間は使用状況（UsageStats）で数えます。制限の表示（ブロック）だけができません。
- 画面オフ・30 分経過・アプリの再起動のどれでも、保護を戻します。
- 使う側のデバイスに、権限を 2 つ用意します（足りないと、「このデバイスの保護」に出ます）。
  1. 「使用状況へのアクセス」を許可する（画面から）。
  2. 設定の書き込みの権限を、PC と USB で 1 回付ける（アンインストールするまで有効。Shizuku のアプリからも付けられます）:
     ```
     adb shell pm grant dev.offsave.app android.permission.WRITE_SECURE_SETTINGS
     ```

## 開発

```sh
cargo test --workspace                       # 判定ロジック・サーバー
cd app && npm install && npm run check && npm test   # 画面の型検査・試験
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
- `app`: Tauri + Svelte の画面。見た目は [@3pino/design-system](https://github.com/3pino/design-system)・[ui-kit-web](https://github.com/3pino/ui-kit-web)・[app-kit-web](https://github.com/3pino/app-kit-web)（disnans と同じ）。色相は 144（`app/src/app.css`）。アイコンはリポジトリ直下の `icon.svg` から `npm run icons`。
- `app/src-tauri/plugins/apk-updater`: Android のアプリ更新（APK を取ってインストーラーを開く）。
- `app/src-tauri/plugins/enforcer`: Android の強制（Kotlin）。`Engine.kt` は `crates/core` の判定の移植で、`crates/core/tests/vectors/engine.json` を両方で試験して一致を保つ。

## 既知の限界

検知も通知もできないもの（[設計メモ](docs/design/overview.md#7-既知の限界検知も通知もできないもの)）:

- Android: 別プロファイル（Work profile・Private Space・アプリのクローン）の中での利用内容。
- 全 OS: 未登録のデバイスでの利用。
- 決済アプリ用の一時停止（上記）の間は、制限を表示できない。ほかの方法で保護が外れたときと違い、通知は「一時停止・再開」として届く。
- ブラウザは、URL バーを読める対応ブラウザだけで、サイトを数える。対応外のブラウザを使うと、友達に通知される。
- 対応ブラウザのうち、実際に確認したのは Chrome と Firefox（161）。ほかは同じ系統の ID で書いてあるが、実機での確認が要る。
