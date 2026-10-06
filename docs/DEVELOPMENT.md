# 開発環境メモ

別のPC(特にWindows)で、LoreShelf を編集・起動できるようにするためのメモ。

## 依存関係はどこに書いてあるか

`requirements.txt` に相当するものは、すでにリポジトリ内にある。

| 役割 | ファイル | 備考 |
|---|---|---|
| フロントエンド(React)の依存 | `app/package.json` と `app/package-lock.json` | `npm install` が読む |
| Rust側の依存 | `app/src-tauri/Cargo.toml` と `app/src-tauri/Cargo.lock` | ビルド時に自動で取得される |

ロックファイル(`package-lock.json`、`Cargo.lock`)はコミットしておくこと。これで、どのPCでも同じバージョンが入る。

ロックファイルに**入らないもの**(Node、Rust、OSのビルドツール)は、下の「事前に入れるもの」を参照。

## 事前に入れるもの

動作確認済みのバージョン(macOS、2026-10-05時点)を併記する。これより新しければ、たいてい問題ない。

| ツール | 確認済み | 入手先 |
|---|---|---|
| Node.js(npm付き) | 25.2.1(npm 11.6.2) | https://nodejs.org |
| Rust(`rustup` 経由) | 1.99.0 | https://rustup.rs |

### Windows で追加で必要なもの

- **Microsoft C++ Build Tools**: Visual Studio Build Tools のインストーラーで「C++ によるデスクトップ開発」を選ぶ。Rustのビルドに必要。
- **WebView2**: Windows 11 には標準で入っている。Windows 10 で入っていなければ、Microsoft の配布ページから入れる。
- Rustは、`rustup` のインストール時に MSVC 版(`x86_64-pc-windows-msvc`)を選ぶ。

詳しい手順は Tauri 公式の「Prerequisites」を参照: https://tauri.app/start/prerequisites/

### macOS で必要なもの

- Xcode(またはコマンドラインツール)。`xcode-select --install`

## 起動までの手順

```sh
git clone <このリポジトリ>
cd LoreShelf/app
npm install
npm run tauri dev
```

初回は、Rust側の依存をビルドするため数分かかる。2回目以降は速い。

## よく使うコマンド(`app/` で実行)

| 目的 | コマンド |
|---|---|
| 開発用に起動(ホットリロード) | `npm run tauri dev` |
| 型チェック(TypeScript) | `npx tsc --noEmit` |
| Rustのテスト | `cd src-tauri && cargo test` |
| 配布用ビルド | `npm run tauri build` |
| 任意のLoreを検証(アプリを起動せずに、エラーと警告を表示) | `cd src-tauri && LORESHELF_CHECK=パス cargo test check_file -- --ignored --nocapture`(Windows PowerShell では `$env:LORESHELF_CHECK="パス"` を先に設定) |

## 動作確認用のデータ

リポジトリに入っているもの(テストもこれを使う):

- `samples/valid/`: 仕様どおりの正しいLore(全8種類のオブジェクトを1つずつ含む作り物)
- `samples/invalid/`: わざと壊したLore。アプリの「ファイルをインポート」で選ぶと、拒否の表示を確認できる

リポジトリに**入っていないもの**:

- `docx/`: 個人のLore(実際の会話から作ったもの)。`.gitignore` で除外していて、公開されない。別のPCでも使いたい場合は、USBやクラウドストレージなどで手動で持っていき、リポジトリ直下に `docx/` として置く。なくても、アプリとテストは動く。

## 詰まりやすいところ

- **「Port 1420 is already in use」**: 前回の開発サーバー(Vite)が残っている。ポートを使っているプロセスを確認して止める。
  - macOS: `lsof -nP -iTCP:1420 -sTCP:LISTEN`
  - Windows(PowerShell): `Get-NetTCPConnection -LocalPort 1420`
- **`cargo` が見つからない**: ターミナルを開き直す。それでもだめなら、PATH に `~/.cargo/bin`(Windowsでは `%USERPROFILE%\.cargo\bin`)を足す。
- **改行コード**: Windows で Git が改行をCRLFに変換することがある。検証は両方の改行に対応しているが、差分が大量に出る場合は `git config core.autocrlf` を確認する。
- 選んだLoreフォルダや、インポートしたファイルのパスは、アプリ内(ブラウザの保存領域)に記憶している。PCごとに別なので、コミットされず、別のPCには引き継がれない。
