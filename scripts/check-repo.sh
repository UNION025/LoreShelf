#!/bin/sh
# 個人の Lore が、リポジトリに入っていないかを調べる(CI から呼ばれる)。
# ローカルでも、  sh scripts/check-repo.sh  で実行できる。

status=0

if git ls-files | grep -Ei '(^|/)docx/' >/dev/null; then
  echo "NG: docx/ の中のファイルが、Git の管理に入っています:"
  git ls-files | grep -Ei '(^|/)docx/'
  status=1
fi

stray=$(git ls-files | grep -Ei '(^|/)lore\.md$')
if [ -n "$stray" ]; then
  echo "NG: LORE.md という名前のファイルが入っています(samples/ には、LORE.md という名前を使いません):"
  echo "$stray"
  status=1
fi

outside=$(git ls-files | grep -Ei '\.(sqlite|sqlite3|db)$')
if [ -n "$outside" ]; then
  echo "NG: データベースのファイルが入っています:"
  echo "$outside"
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "OK: 個人の Lore らしいファイルは、見つかりませんでした。"
fi
exit $status
