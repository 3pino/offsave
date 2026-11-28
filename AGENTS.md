# offsave

友達どうしで見守るスクリーンタイムのアプリ。利用・運用は [README.md](README.md)、開発・リリースの手順は [CONTRIBUTING.md](CONTRIBUTING.md)、設計の決定事項は [docs/design/overview.md](docs/design/overview.md)。

## 構成

- `crates/core`（Rust）: 判定ロジックとサーバーとの型。
- `crates/server`（Rust）: axum + SQLite のサーバー。
- `app`（Tauri + Svelte）と `app/src-tauri/plugins/enforcer`（Kotlin）: Android アプリ。

## 決まり

- 判定ロジック（`crates/core/src/engine.rs`）を変えたら、Kotlin の `Engine.kt` も同じに直し、`crates/core/tests/vectors/engine.json` に試験を足す（Rust と Kotlin の両方が同じファイルで通ること）。
- 制限の強制は、必ず端末の中で行う（サーバーは方針の配布・集計・通知）。
- URL は、ホスト名だけを扱う。パス・クエリ・フラグメントは、読んだ時点で捨てる（保存も送信もしない）。
- 監視を弱める操作（無効化・解除・端末の削除など）は、必ず友達に通知する。例外: 友達が決めたアプリのための監視の一時停止・再開（状態はデバイスの一覧に出る）。
- 画面の文言は日本語。「端末」ではなく「デバイス」と書く。
- 画面の部品・色は、disnans と同じ共有パッケージ（`@3pino/design-system`・`ui-kit-web`・`app-kit-web`）を使う。色の直書きはしない（トークンを使う）。色相は `app/src/app.css` の `--hue`（144）で、アイコン（`app/scripts/make-icons.mjs`）と同じ値にする。
- ナビゲーションは「ホーム・ルール・友達・通知・設定」。ルールは 1 つのタブにまとめ、ルールは複数の制限（`limits`）を持つ。
- 署名の鍵（`~/.config/offsave-keys/`）と、GitHub の Secrets は、リポジトリに入れない・出力に出さない。リリースの手順は CONTRIBUTING.md の「リリースの作り方」。README.md には、利用・運用に必要なことだけを書く（開発の話は CONTRIBUTING.md へ）。
- 見た目は、実機・エミュレーターで人が最終確認する（スクリーンショットでの確認は補助）。

## テスト

```sh
scripts/check-typos.sh   # 誤字・文言の決まり。誤字を直したら scripts/typos.tsv に足す
cargo test --workspace && cargo clippy --workspace --all-targets
cd app && npm run check && npm test
cd app/src-tauri/gen/android && ./gradlew :tauri-plugin-enforcer:testDebugUnitTest :tauri-plugin-apk-updater:testDebugUnitTest
```

## ビルド

Android のビルドは `npx tauri android build --debug --target x86_64 --apk`（`app` で実行。`ANDROID_HOME`・`NDK_HOME`・`JAVA_HOME` が要る）。
`gen/android` は `tauri android init` の生成物を commit している（環境依存のファイルは `.gitignore`）。
