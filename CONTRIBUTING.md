# 開発の手引き

offsave の開発・リリースの手順です。利用・運用の説明は [README.md](README.md)、設計の決定事項は [docs/design/overview.md](docs/design/overview.md)にあります。

## 構成

- `crates/core`: ルールの判定（猶予・一時解除・日付の境界）、利用区間の和集合、サーバーとの型。
- `crates/server`: axum + SQLite。REST と、デバイスの WebSocket（`/api/ws/device`）。環境変数は [crates/server/README.md](crates/server/README.md)。
- `app`: Tauri + Svelte の画面。見た目は [@3pino/design-system](https://github.com/3pino/design-system)・[ui-kit-web](https://github.com/3pino/ui-kit-web)・[app-kit-web](https://github.com/3pino/app-kit-web)（disnans と同じ）。色相は 144（`app/src/app.css`）。アイコンはリポジトリ直下の `icon.svg` から `npm run icons`。
- 戻る操作（Android の戻る・PC の Alt+←）: `ui.pushLayer` で、開いているサブ画面・ダイアログを積む。戻ると、新しいものから 1 つずつ閉じ、何も開いていなければ、ホーム以外のタブからホームへ戻り、ホームなら、アプリが裏に回る。サブ画面を作るときは、`$effect` の中で `ui.pushLayer` を呼び、返り値を返す（`Settings.svelte` を参照）。
- `app/src-tauri/plugins/apk-updater`: Android のアプリ更新（APK を取ってインストーラーを開く）。
- `app/src-tauri/plugins/enforcer`: Android の強制（Kotlin）。`Engine.kt` は `crates/core` の判定の移植で、`crates/core/tests/vectors/engine.json` を両方で試験して一致を保つ。

仕組み:

```
Android ── WebSocket（60 秒ごとのハートビート）──▶ サーバー（SQLite）◀── 友達のデバイス
   │  Accessibility で前面のアプリ・サイトを検知                │  ルール・利用の集計・イベント
   └─ 制限に達したら、全画面で知らせる → 猶予 1 分 → ブロック   └─ ハートビートが途絶えたら友達に通知
```

## テスト

```sh
scripts/check-typos.sh                       # 誤字・文言の決まり（誤字を直したら scripts/typos.tsv に足す）
cargo test --workspace && cargo clippy --workspace --all-targets   # 判定ロジック・サーバー
cd app && npm install && npm run check && npm test                 # 画面の型検査・試験
cd app/src-tauri/gen/android && ./gradlew :tauri-plugin-enforcer:testDebugUnitTest :tauri-plugin-apk-updater:testDebugUnitTest
                                             # Android の判定・追跡（Rust と共通の試験データ）
```

判定ロジック（`crates/core/src/engine.rs`）を変えたら、Kotlin の `Engine.kt` も同じに直し、`engine.json` に試験を足します。

## ビルド

Android（x86_64 のエミュレーター向けのデバッグ版）:

```sh
cd app
export ANDROID_HOME=$HOME/Android/Sdk NDK_HOME=$ANDROID_HOME/ndk/<版> JAVA_HOME=/usr/lib/jvm/java-25-openjdk-amd64
npx tauri android build --debug --target x86_64 --apk       # 実機（arm64）は --target aarch64
```

開発用のサーバー（エミュレーターからは `http://10.0.2.2:8090`）:

```sh
OFFSAVE_DEV=1 OFFSAVE_BIND=127.0.0.1:8090 OFFSAVE_DATA_DIR=/tmp/offsave-dev cargo run -p offsave-server
```

## 更新の仕組み

- **Android**: アプリの「設定」の一番下の「バージョン」が、GitHub の最新リリースを見て、APK を取ってインストーラーを開く。新しい版があると、トーストと「設定」の点で知らせる。APK はリリースの鍵で署名してあり、Android のインストーラーが「入っているアプリと同じ鍵」でないと入れない（ダウンロード元もこのリポジトリのリリースに限る）。
- **Windows / Linux（デスクトップ）**: Tauri のアップデーター。`latest.json` を見て、署名（minisign）を確かめてから入れる。公開鍵はアプリに入っている。
- **サーバー**: `OFFSAVE_AUTO_UPDATE=1` で、最新の正式版を 10 分おきに見て、sha256 と `--version` を確かめ、DB をバックアップしてから入れ替えて再起動する。

## リリースの作り方

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
