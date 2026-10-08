# 壊れたサンプル

LoreShelf が LoreSpec v0.1 からのずれを**診断して見せる**ことを確かめるための、作り物のファイルです。
すべて `../valid/all-object-types.md`(正しいサンプル)に、1か所だけ手を加えて作っています。
アプリの「ファイルをインポート」で選ぶと、診断の表示(バッジ、トースト、詳細の一覧)を確認できます。

LoreShelf は、ファイルが読めるかぎり**読み込んで、表示して、検索できるようにします**(診断は結果に添えるだけです)。
下の「診断」は、どんなずれとして報告されるかを表します。

| ファイル | 壊し方 | 診断 |
|---|---|---|
| `translated-qualifier.md` | `Qualifier` の値を日本語に翻訳した。公式の例も `Qualifier` に文章を書いているため、重くは扱わない | 警告のみ |
| `bad-connections.md` | 不明な接続種類 `causes`、書式が不正な行、存在しないID `D99` への参照 | エラー |
| `bad-enums-and-ids.md` | `Origin: adapted`、`Status: finalized`、同じセクション内のIDの重複 | エラー |
| `mixed-up-enums.md` | 別のオブジェクト型の値を取り違えた(Artifact の `Status` に Decision の値 `provisional`、Pattern の `Origin` に Insight の `Source` の値 `research`)。AIが実際に出力した間違いを、作り物の内容で再現したもの | エラー |
| `no-frontmatter.md` | フロントマターがない(タイトルはファイル名で代用される) | エラー |
| `unsupported-version.md` | `lorespec: "0.2"` | エラー |
| `warnings-only.md` | `Warrant` の欠落と、仕様にないセクション | 警告のみ |

このフォルダのファイルは、`app/src-tauri/src/lore.rs` のテストが「意図した理由で診断される」ことを確認しています。
