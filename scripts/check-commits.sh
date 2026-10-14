#!/bin/sh
# AI ツールを共同作者にする行(Co-authored-by)が、コミットに入っていないかを調べる。
#
#   sh scripts/check-commits.sh <基点> <終点>      コミットの範囲を調べる(CI が使う)
#   sh scripts/check-commits.sh --message-file F   コミットメッセージ1つを調べる(commit-msg フックが使う)

pattern='^[[:space:]]*co-authored-by:.*(claude|anthropic|copilot|openai|chatgpt|gpt-|gemini|codex|cursor|devin|aider|\[bot\])'

if [ "$1" = "--message-file" ]; then
  if grep -Eiq "$pattern" "$2"; then
    echo "NG: AI ツールを共同作者にする Co-authored-by の行があります。消してください。"
    echo "    AI を使ったことは、プルリクエストの本文に書きます(CONTRIBUTING.md を見てください)。"
    grep -Ei "$pattern" "$2"
    exit 1
  fi
  exit 0
fi

status=0
for c in $(git rev-list "$1..$2"); do
  if git log -1 --format=%B "$c" | grep -Eiq "$pattern"; then
    echo "NG: コミット $c に、AI ツールの Co-authored-by の行があります:"
    git log -1 --format=%B "$c" | grep -Ei "$pattern"
    status=1
  fi
done

if [ "$status" -eq 0 ]; then
  echo "OK: AI ツールを共同作者にするコミットは、見つかりませんでした。"
else
  echo ""
  echo "直し方: git rebase -i で該当のコミットを reword し、その行を消して、force push します。"
  echo "分からなければ、プルリクエストのコメントで聞いてください。"
fi
exit $status
