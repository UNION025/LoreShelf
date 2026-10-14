# LoreShelf への参加のしかた

> **English summary:** LoreShelf is a local-first desktop app for storing, searching and reusing LoreSpec knowledge (`LORE.md`) from AI conversations. The project is currently run in Japanese: documents and prompts are written in Japanese first, and English versions will follow. Contributions are welcome; please open an issue first (Japanese or English). Key rules: never commit personal Lore (the `docx/` folder is ignored on purpose), use your GitHub noreply email address, using AI tools is welcome but please say so in the pull request, and do not add AI tools as `Co-authored-by`. Proposals to change LoreSpec itself go through [docs/EXTENSIONS.md](docs/EXTENSIONS.md) here first.

LoreShelf に興味を持ってくれて、ありがとうございます。
このプロジェクトは、**Git や GitHub の使い方を学ぶ場所**としても、使ってもらうことを、考えています。分からないことは、遠慮なく、issue で聞いてください。

## どんな参加ができるか

コードが書けなくても、参加できます。

- **文書**: 説明の分かりにくいところの修正、手順の追加、誤字の直し。
- **プロンプトの研究**: AI に Lore を作らせるための指示文(`docs/PROMPTS.md`)を、試して、結果を報告する。分野ごとの特化プロンプトの案を出す。
- **サンプル**: 動作確認に使う、**作り物の**Lore(`samples/`)。
- **不具合の報告・提案**: issue に書く。

プログラム(`app/`)の修正は、まず issue で、相談してください。

## 最初にやること

1. 変更したい点を、**issue に書く**(小さな誤字の直しなら、省略してかまいません)。
2. GitHub の画面で、ファイルを開いて、鉛筆のボタンで編集し、変更を提案する(Git のコマンドは、いりません)。これが、プルリクエストになります。
3. 慣れたら、GitHub Desktop や、コマンドでの操作に、進んでください。

## 必ず守ってほしいこと

### 1. 個人の Lore を、リポジトリに入れない
LoreShelf の Lore は、AI との**個人的な会話**から作られます。名前、連絡先、未発表の内容が入っていることがあります。

- 自分の Lore を置く `docx/` フォルダは、**わざと、Git の管理から外してあります**。
- `samples/` に置いてよいのは、**作り物(架空の内容)だけ**です。実際の会話から作ったものを、少し直して、置くのも、やめてください。
- **もし、入れてしまったら、すぐに、メンテナーに連絡してください。** あとから、ファイルを消す変更を、重ねても、**履歴には、残ります**。履歴の書き換えが必要になるので、早いほど、小さく済みます。
- CI(自動検査)が、`docx/` の中のファイルや、`LORE.md` という名前のファイル、データベースのファイルを見つけると、プルリクエストが、止まります。ただし、これは**名前での簡単な検査**です。中身までは見られないので、最後は、出す人の確認が頼りです。

### 2. メールアドレスを守る
コミットには、メールアドレスが、残ります。**本物のメールアドレスを、公開しない**ために、GitHub で、次の2つを、オンにしてください(Settings の Emails)。

- `Keep my email addresses private`
- `Block command line pushes that expose my email`

コミットに使うメールは、GitHub が用意する `〔数字〕+〔ユーザー名〕@users.noreply.github.com` の形にします。

### 3. AI を使ってよい。使ったことは、正直に書く
AI ツールを使って、書いたり、直したりすることは、**歓迎**です。そのうえで、お願いが4つあります。

- **使ったことを、プルリクエストに書く**(ひな型に、欄があります。どの AI かは、任意です)。使わない人もいるので、**どちらが、よい悪いでもありません**。透明にしておくための、記録です。
- **AI の出力は、自分で読んで、確認してから、出す。** 内容の責任は、出した人にあります。
- **AI に、個人情報や、未発表の内容を、入力しない。**
- **`Co-authored-by:` などの行で、AI ツールを、共同作者にしない。** GitHub は、この行のあるアカウントを、リポジトリの Contributors に、載せます。AI を使わない人も、いるので、一覧に、AI が並ぶのは、避けたいためです。使ったことは、プルリクエストの本文に、書いてください。
  - **自動で入ることがあります。** 一部の AI ツール(Claude Code、GitHub Copilot など)は、コミットに、この行を**勝手に足します**。出す前に、`git log` で確認してください。
  - **CI が止めます。** AI ツールの `Co-authored-by` を見つけると、プルリクエストの検査が、失敗します(`scripts/check-commits.sh`)。
  - **手元でも止められます。** 次を一度実行すると、コミットの時点で、止まります。
    ```sh
    git config core.hooksPath .githooks
    ```

### 4. プロンプトを変えたら、版とタグも直す
`docs/PROMPTS.md` を変えるときは、冒頭の**版**を上げ、**プロンプトの中のタグ**(`loreshelf-prompts-v…`)を、すべて、新しい版に直します。版とタグの不一致は、テストが見つけます(`cargo test`)。履歴の表にも、1行、足してください。詳しくは、`docs/PROMPTS.md` の「版の履歴」を、見てください。

### 5. LoreSpec 自体への提案は、まず、ここに
LoreShelf は、[LoreSpec](https://github.com/lorespec-org/lorespec) に沿っています。LoreSpec の仕様に足したいことが、見つかったときは、**LoreSpec に、直接、提案せず**、まず、**このリポジトリに、issue(またはプルリクエスト)で、出してください**。書き方は、[docs/EXTENSIONS.md](docs/EXTENSIONS.md) に、あります。上流(LoreSpec)への提案は、メンテナーが、まとめます。

## プルリクエストの流れ

1. **小さく、1つの目的**にする。
2. ひな型の欄を、埋める(変更の内容、確認したこと、個人情報が入っていないこと、AI を使ったか)。
3. CI(自動検査)が、通るのを、待つ。通らないときは、画面の説明を、読んで、直す。分からなければ、コメントで聞く。
4. メンテナーが、確認します。**返事が、遅れることがあります**。急ぎではない、という前提で、お願いします。

## 開発環境(コードを触る場合)

[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) に、手順があります。出す前に、次を、実行してください。

```sh
cd app
npm install
npx tsc --noEmit
cd src-tauri
cargo test
```

## 権限について(書き込み権限)

このリポジトリでは、**招待制**で、書き込み権限を、渡します。最初は、**フォークして、プルリクエストを出す**形で、参加してください。

目安は、次のとおりです(最終的な判断は、メンテナーが、行います)。

- 手直しをほとんど、必要としない、プルリクエストが、いくつかマージされている。
- 他の人のプルリクエストに、役に立つレビューを、している。
- 上の「必ず守ってほしいこと」を、理解している。

まず、issue と、プルリクエストの**整理だけ**をする役割(Triage)から、始めることもあります。

## ライセンス

このリポジトリは、[MIT ライセンス](LICENSE)です。プルリクエストで、提出されたものは、同じ MIT ライセンスで、公開されることに、同意したものとします。`docs/PROMPTS.md` に載せている、LoreSpec の Scribe プロンプトは、LoreSpec(MIT)の著作物で、ライセンス表記を、残しています。変えないでください。

## 言語

当面は、**日本語が、正本**です。英語版は、あとから、追加します(AI の翻訳の補助を、使うことがあります。そのときは、「日本語が、正本」と、明記します)。英語で、issue を書いても、かまいません。
