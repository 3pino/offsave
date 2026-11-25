# offsave

友達どうしで見守るスクリーンタイムのアプリ。概要・動かし方は [README.md](README.md)、設計の決定事項は [docs/design/overview.md](docs/design/overview.md)。

## 構成

- `crates/core`（Rust）: 判定ロジックとサーバーとの型。
- `crates/server`（Rust）: axum + SQLite のサーバー。
- `app`（Tauri + Svelte）と `app/src-tauri/plugins/enforcer`（Kotlin）: Android アプリ。

## 決まり

- 判定ロジック（`crates/core/src/engine.rs`）を変えたら、Kotlin の `Engine.kt` も同じに直し、`crates/core/tests/vectors/engine.json` に試験を足す（Rust と Kotlin の両方が同じファイルで通ること）。
- 制限の強制は、必ず端末の中で行う（サーバーは方針の配布・集計・通知）。
- URL は、ホスト名だけを扱う。パス・クエリ・フラグメントは、読んだ時点で捨てる（保存も送信もしない）。
- 保護を弱める操作（無効化・解除・端末の削除など）は、必ず友達に通知する。
- 画面の文言は日本語。「端末」ではなく「デバイス」と書く。
- 見た目は、実機・エミュレーターで人が最終確認する（スクリーンショットでの確認は補助）。

## テスト

```sh
cargo test --workspace && cargo clippy --workspace --all-targets
cd app && npm run check
cd app/src-tauri/gen/android && ./gradlew :tauri-plugin-enforcer:testDebugUnitTest
```

## ビルド

Android のビルドは `npx tauri android build --debug --target x86_64 --apk`（`app` で実行。`ANDROID_HOME`・`NDK_HOME`・`JAVA_HOME` が要る）。
`gen/android` は `tauri android init` の生成物を commit している（環境依存のファイルは `.gitignore`）。
