# LoreShelf

**AIとの会話から生まれた知識を、手元に残すためのローカルファーストなライブラリ。**

[English](./README.md) | **日本語**

> [!NOTE]
> LoreShelfは現在、初期開発・設計段階のプロジェクトです。  
> 仕様や機能、ファイル構造などは今後変更される可能性があります。

LoreShelfは、[LoreSpec](https://github.com/lorespec-org/lorespec) の `LORE.md` を保存・整理・検索し、過去のAIとの会話から得られた知識を再利用するためのローカルファーストなアプリケーションです。

LoreSpecは、AIとの会話から得られた知識を、構造化されたポータブルな形式で保存するためのオープンな仕様です。

LoreShelfは、その**「次」**を扱います。

> **`LORE.md` が数百、数千と蓄積されたとき、それらをどう管理し、どう見つけ、どう再利用するのか？**

LoreShelfは `LORE.md` を個人のライブラリとして整理し、全文検索や意味検索によって、過去のAIとの会話から得られた知識を再発見できるようにすることを目指しています。

元の `LORE.md` はLoreShelf独自のデータ形式に閉じ込めず、ポータブルな状態を維持します。

## なぜLoreShelf？

AIと長く会話していると、

> 「これ、前にもAIに聞いた気がする……」

ということがあります。

しかし、その会話が数か月前だったり、正確な単語を覚えていなかったり、そもそもChatGPT・Claude・Geminiなど、どのAIで話したのか覚えていないこともあります。

現在のチャット履歴は、こうした過去の知識を長期間蓄積して再利用する用途には必ずしも適していません。

LoreShelfでは、たとえば、

> 「前に動的IPについて話した気がする」

のような曖昧な記憶からでも、関連するLoreを探せることを目指します。

見つけたLoreから、

- そのとき何を調べたのか
- 何が分かったのか
- どんな判断をしたのか
- 何が未解決だったのか

を確認できます。

さらに、そのLoreをChatGPT、Claude、GeminiなどのAIへ再びコンテキストとして渡すことで、以前の会話を土台にして続きを話すこともできます。

**保存 → 再発見 → 継続**

これがLoreShelfの基本的な考え方です。

## LoreSpecを基盤に

LoreShelfは、新しい独自のAI会話保存形式を一から定義するのではなく、LoreSpecを知識フォーマットとして利用します。

役割は明確に分けます。

```text
LoreSpec
│
├─ LORE.md の構造
├─ Knowledge Objects
├─ Connections
├─ Trails
└─ Scribe
        │
        ▼
     LORE.md
        │
        ▼
    LoreShelf
        │
        ├─ Library
        ├─ Full-text Search
        ├─ Semantic Search
        ├─ Knowledge Browser
        ├─ Lineage
        └─ Import / Export
```

**LoreSpecは「AIとの会話から何を残すか」を定義します。**

**LoreShelfは「残したLoreをどう蓄積し、探し、再利用するか」を扱います。**

LoreShelfは独立したプロジェクトであり、LoreSpec公式のアプリケーションではありません。

## ローカルファースト

AIとの会話を何年も蓄積してできた知識が、特定のWebサービスやサーバーの存続に依存するべきではないと考えています。

そのためLoreShelfは、ローカル環境だけでも利用できることを基本方針とします。

```text
LORE.md
LORE.md
LORE.md
   │
   ▼
┌─────────────────────┐
│      LoreShelf      │
│                     │
│  Local Library      │
│  Local Database     │
│  Search Index       │
│  Embeddings         │
│  Lineage            │
└─────────────────────┘
```

ローカルデータベースは、高速な検索・整理・意味検索などを行うための管理・インデックス層として利用します。

**LoreShelfのデータベースだけが知識の唯一の正本になることは目指しません。**

元となるLoreはポータブルな形で保持し、データベースを再構築したり、LoreShelf以外のツールへ移行したりできる設計を目指します。

## 意味検索

LoreShelfでは、単純なキーワード検索だけでなく、意味検索（Semantic Search）への対応を予定しています。

たとえば過去のLoreに、

> Dynamic DNS can be used to keep a hostname pointing to a changing public IP address.

と保存されていたとしても、

> 「前に動的IPの対策について聞いた気がする」

のような検索から発見できることを目指します。

これにより、正確なタイトルやキーワードを覚えていなくても、過去の知識へ戻れるようにします。

## TrailsとLineage

LoreSpecにはすでに **Trails** という概念があります。

Trailは、複数のSessionにまたがる関連Knowledge Objectを結び、あるテーマについて思考がどのように発展したのかを表現します。

LoreShelfでは、それとは別に **Lineage（系譜）** という概念を検討しています。

両者は似ていますが、扱うものが異なります。

**Trail**

> 「これらの知識・アイデアはどのようにつながっているか？」

**Lineage**

> 「この会話は、以前のどの会話のコンテキストを実際に引き継いで生まれたのか？」

たとえば、

```text
Lore A
  │
  │ AをAIへ読み込ませて続きを会話
  ▼
Lore B
  │
  │ BをAIへ読み込ませて続きを会話
  ▼
Lore C
```

なら、A → B → C は同じLineageです。

一方で、

```text
Lore A ──→ Lore B ──→ Lore C


Lore X
```

Lore XがA/B/Cとほぼ同じ話題について扱っていたとしても、A/B/Cのコンテキストを実際に引き継いでいないのであれば、同じLineageにはしません。

つまり、

> **意味的に似ていることと、同じ系譜であることは別です。**

意味検索やLoreSpecのConnections / Trailsは「関連性」を見つけるために使えます。

Lineageは「実際のコンテキスト継承」を記録するために使います。

## 想定している機能

初期段階では、以下のような機能を検討しています。

- `LORE.md` のインポート
- ローカルLoreライブラリ
- Lore一覧・詳細表示
- Knowledge Objectの閲覧
- 全文検索
- 意味検索
- Trailsの閲覧・可視化
- Lineageの記録・可視化
- Loreのインポート / エクスポート
- ローカルデータベースの再構築
- LoreSpecバージョンの管理

将来的には、ブラウザ拡張やAIサービスとの連携、Loreの共有なども検討する可能性があります。

ただし、オンラインサービスへの接続を必須にはせず、LoreShelf単体で個人のLoreライブラリを管理できることを重視します。

## データのポータビリティ

LoreShelfの重要な設計目標のひとつは、**LoreShelfを使わなくなってもLoreが残ること**です。

LoreShelfそのものが開発終了した場合でも、保存した知識まで失われるべきではありません。

そのため、

```text
Lore
 ↓
LoreShelf
 ↓
LoreShelf独自DBにロックイン
```

ではなく、

```text
             ┌─ LoreShelf
             │
LORE.md ─────┼─ 他のLoreSpec対応ツール
             │
             └─ AIへ直接読み込み
```

という関係を目指します。

## ステータス

🚧 **Early Development / Experimental**

LoreShelfは現在、設計および初期開発段階です。

特に以下の仕様はまだ確定していません。

- Lineageの表現方法
- LoreShelf固有メタデータの形式
- ローカルデータベース構造
- Embedding / Semantic Searchの実装
- LoreSpecの将来バージョンへの追従方法
- AIへの再投入・継続方法

実際の `LORE.md` を使った検証を行いながら設計していく予定です。

## LoreSpec

LoreShelfは [LoreSpec](https://github.com/lorespec-org/lorespec) を基盤として開発されています。

LoreSpecについて詳しくは、公式リポジトリおよび仕様書を参照してください。

- [LoreSpec GitHub](https://github.com/lorespec-org/lorespec)
- [LoreSpec v0.1 Specification](https://github.com/lorespec-org/lorespec/blob/main/spec/v0.1/SPEC.md)
- [Scribe](https://github.com/lorespec-org/lorespec/blob/main/tools/scribe.md)

LoreShelfはLoreSpecとは独立したプロジェクトであり、LoreSpecプロジェクトによる公式・公認アプリケーションではありません。

## License

MIT License

詳細は [LICENSE](LICENSE) を参照してください。

LoreShelfは、同じくMITライセンスで公開されている [LoreSpec](https://github.com/lorespec-org/lorespec)(Copyright (c) 2026 LoreSpec Contributors)を基盤としています。[docs/PROMPTS.md](docs/PROMPTS.md) には、LoreSpecのScribeプロンプトを原文のまま、LoreSpecのライセンス表記とともに掲載しています。
