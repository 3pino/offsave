#!/usr/bin/env bash
# 使い方: scripts/bump-version.sh 0.3.0
# 版を、すべての場所（アプリ・サーバー・Tauri の設定）で揃える。タグ（v0.3.0）は、コミットしてから打つ。
set -euo pipefail
V=${1:?版を指定してください（例: 0.3.0）}
[[ $V =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "版は 1.2.3 の形で指定してください" >&2; exit 1; }
cd "$(dirname "$0")/.."

# JSON と Cargo.toml の version を書き換える（最初の version = の行だけ。サーバー・コアはルートの [workspace.package]）
tmp=$(mktemp)
jq --arg v "$V" '.version = $v' app/src-tauri/tauri.conf.json > "$tmp" && mv "$tmp" app/src-tauri/tauri.conf.json
jq --arg v "$V" '.version = $v' app/package.json > "$tmp" && mv "$tmp" app/package.json
for f in Cargo.toml app/src-tauri/Cargo.toml; do
  perl -0pi -e 's/^version = "[^"]*"/version = "'"$V"'"/m' "$f"
done
echo "版を $V にしました。Cargo.lock を更新します"
cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null
(cd app/src-tauri && cargo update -p offsave-app --offline >/dev/null 2>&1 || cargo update -p offsave-app >/dev/null)
git diff --stat
