# 壊れたサンプル

LoreShelf が LoreSpec v0.1 違反を検出して**読み込みを拒否する**ことを確かめるための、作り物のファイルです。
すべて `../valid/all-object-types.md`(正しいサンプル)に、1か所だけ手を加えて作っています。
アプリの「ファイルをインポート」で選ぶと、拒否の表示を確認できます。

| ファイル | 壊し方 | 結果 |
|---|---|---|
| `translated-qualifier.md` | `Qualifier` の値を日本語に翻訳した | 拒否 |
| `bad-connections.md` | 不明な接続種類 `causes`、書式が不正な行、存在しないID `D99` への参照 | 拒否 |
| `bad-enums-and-ids.md` | `Origin: adapted`、`Status: finalized`、同じセクション内のIDの重複 | 拒否 |
| `no-frontmatter.md` | フロントマターがない | 拒否 |
| `unsupported-version.md` | `lorespec: "0.2"` | 拒否 |
| `warnings-only.md` | `Warrant` の欠落と、仕様にないセクション | 受理(警告のみ) |

このフォルダのファイルは、`app/src-tauri/src/lore.rs` のテストが「意図した理由で拒否される」ことを確認しています。
