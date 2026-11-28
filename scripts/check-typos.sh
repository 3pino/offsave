#!/usr/bin/env bash
# 誤字と、文言の決まり（AGENTS.md）の違反を探す。見つかったら 1 で終わる。
#
#   scripts/check-typos.sh
#
# 探す語は scripts/typos.tsv にある（種類<TAB>誤<TAB>正）。
# - typo: git で管理しているすべてのファイルから探す。
# - word: 画面の文言を持つファイルのコメント以外の行から探す（コメントの中の「端末」などは見ない）。
set -euo pipefail
cd "$(dirname "$0")/.."

LIST=scripts/typos.tsv
# 画面・通知の文言があるところ
WORD_FILES=(
  'app/src/*.svelte' 'app/src/*.ts'
  'app/src-tauri/plugins/enforcer/android/src/main/*.kt'
  'crates/server/src/*.rs' 'crates/core/src/*.rs'
)

found=0

while IFS=$'\t' read -r kind bad good; do
  [[ -z "${kind}" || "${kind}" == \#* ]] && continue

  case "${kind}" in
    typo)
      # このファイルと一覧そのものは、誤字を書いてあるので除く
      hits=$(git ls-files -z | grep -zvE '^scripts/(typos\.tsv|check-typos\.sh)$' \
        | xargs -0 grep -nIF -- "${bad}" 2>/dev/null || true)
      ;;
    word)
      hits=$(git ls-files -z -- "${WORD_FILES[@]}" \
        | xargs -0 grep -nIF -- "${bad}" 2>/dev/null \
        | grep -Ev '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*)' || true)
      ;;
    *)
      echo "typos.tsv の種類が不明です: ${kind}" >&2
      exit 2
      ;;
  esac

  if [[ -n "${hits}" ]]; then
    echo "「${bad}」→「${good}」（${kind}）"
    echo "${hits}" | cut -c1-200 | sed 's/^/  /'
    found=1
  fi
done < "${LIST}"

if [[ "${found}" -eq 0 ]]; then
  echo "誤字・文言の違反はありません。"
fi
exit "${found}"
