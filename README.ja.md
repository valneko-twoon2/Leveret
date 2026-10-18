<div align="center">

<img src="backend/icons/leveret.svg" alt="Leveret" width="128" height="128">

# Leveret

**Windows 向けの、小さくて扱いやすい Git クライアント。**<br>
一覧からリポジトリを選んで、変更を確かめて、コミット。それだけ。

[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white)](#必要なもの)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-2024-CE422B?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)
[![License: MIT](https://img.shields.io/badge/license-MIT-f2665a)](LICENSE)

[English](README.md) · **日本語**

</div>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/dark/02-changes.png">
  <img alt="変更画面: チェック付きのファイル一覧、差分、コミットフォーム" src="docs/screenshots/light/02-changes.png">
</picture>

## 機能

| | |
|---|---|
| **リポジトリ一覧** | フォルダ選択やドラッグ＆ドロップで登録し、ワンクリックで切り替え。 |
| **GitHub サインインとクローン** | ブラウザ（またはトークン）でサインインし、自分のリポジトリを選んで進捗バー付きでクローン。 |
| **選んだファイルだけコミット** | 含めるファイルにチェックを入れ、概要と説明を書いて `Ctrl+Enter`。未追跡のファイルは区別して表示し、非表示にもできます。.gitignore で無視したファイルは表示されません。 |
| **フェッチ / プル / プッシュ** | ツールバーのボタン1つで、そのとき必要な操作（プル、プッシュ、公開、同期）を進捗付きで実行。10 分ごとにバックグラウンドでフェッチ。 |
| **ブランチとタグ** | 切り替え、作成、マージ、リベース、削除を1つのダイアログで。リモートブランチは追跡ブランチとしてチェックアウト。履歴からタグ（軽量 / 注釈付き）を作成し、プッシュや削除もできます。 |
| **やり直しやすい** | 破棄した変更はごみ箱へ。直前のコミットの修正や取り消し、任意のコミットのリバートやチェリーピック。 |
| **競合が分かりやすい** | 止まったマージ・リベース・リバートをバナーで表示し、解決済みにする / 続行 / スキップ / 中止。ファイルごとに片方の内容を採用したり WinMerge でマージしたりでき、競合マーカーが残ったまま解決済みにしようとすると確認します。 |
| **スタッシュ** | 作業中の変更を一時的に退避して、あとで戻せます。 |
| **Issue** | GitHub の Issue とローカルの Issue を並べて扱えます。まずローカルに書き留め、必要になったら GitHub に反映（任意）。 |
| **設定** | テーマ、Git の名前・メールやプルの方法、差分の表示、バックグラウンドフェッチ、クローン先、AI エージェントの権限、Leveret と同梱しているオープンソースソフトウェアのライセンス（`Ctrl+,`）。 |
| **使いやすさ** | 各領域の幅を変更可能、右クリックメニュー、キーボードショートカット、履歴の検索。 |
| **読みやすい差分** | 行番号・ハンク見出し付き。Shift_JIS のファイルも文字化けせずに表示。 |
| **WinMerge** | 差分を WinMerge で開けます（ボタン、ダブルクリック、右クリック）。競合したファイルは 3 方向マージで開きます。WinMerge がインストールされているときに表示されます。 |
| **グラフ付きの履歴** | ブランチのレーン、ブランチ / リモート / タグのラベル、コミット詳細とファイルごとの差分。 |
| **ライト / ダーク** | Windows のテーマに追従。 |
| **多言語化に対応した作り** | UI の文言はすべて型付きのメッセージカタログで管理（現在は日本語）。 |
| **撮影機能を内蔵** | 全画面をバックグラウンドで撮影。ドキュメントやレビューに便利。 |
| **しっかり封じ込め** | ブラウザのタブではなくデスクトップアプリとして動作。開発者ツール・再読み込み・印刷・ズームは無効、バックエンドで入力を厳しく検証。 |
| **AI エージェント対応** | MCP サーバーを内蔵。Claude Code、Codex、GitHub Copilot などのエージェントがすべての機能を使えます。 |
| **落ちにくい** | git の操作が重なることはなく、表示のエラーはその場で復帰。クラッシュはログに残ります。 |

> [!NOTE]
> Leveret はインストール済みの `git.exe` を呼び出すので、git の設定・資格情報ヘルパー・SSH 鍵・フックはコマンドラインと同じように動きます。

## スクリーンショット

<table>
  <tr>
    <td width="50%">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/dark/04-history.png">
        <img alt="History view with the branch graph" src="docs/screenshots/light/04-history.png">
      </picture>
      <p align="center">履歴 — グラフ、ラベル、コミットの詳細</p>
    </td>
    <td width="50%">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/dark/19-tags.png">
        <img alt="Tags tab" src="docs/screenshots/light/19-tags.png">
      </picture>
      <p align="center">タグ — チェックアウト、プッシュ、削除</p>
    </td>
  </tr>
  <tr>
    <td width="50%">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/dark/13-rebase-conflict.png">
        <img alt="A rebase stopped on conflicts" src="docs/screenshots/light/13-rebase-conflict.png">
      </picture>
      <p align="center">競合 — 解決、続行、中止</p>
    </td>
    <td width="50%">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/dark/21-settings.png">
        <img alt="Settings screen" src="docs/screenshots/light/21-settings.png">
      </picture>
      <p align="center">設定</p>
    </td>
  </tr>
</table>

ほかの画面: [全画面のスクリーンショット（ライト / ダーク）](docs/screenshots.ja.md)

## 仕組み

```mermaid
flowchart LR
    subgraph UI["frontend/ — Svelte 5"]
        C[コンポーネント] --> S[ストア *.svelte.ts]
        S --> A[api.ts]
        A -. ブラウザ / --mock .-> M[mock/]
    end
    subgraph Core["backend/ — Rust + Tauri 2"]
        V{{入力の検証}} --> G[git ラッパー<br>status · diff · log · commit<br>remote · branch · rebase · stash]
        V --> R[リポジトリ一覧<br>repos.json]
        V --> H[GitHub API<br>デバイスフロー · リポジトリ一覧]
        V --> P[WebView2 撮影]
        T[エージェント用ツール] --> V
    end
    A -- invoke --> V
    AG[AI エージェント<br>Claude Code · Codex · Copilot] -- "MCP (stdio)" --> T
    G -- 起動 --> X[(git.exe)]
    H -- HTTPS --> GH[(api.github.com)]
    H -. トークン .-> K[(Windows<br>資格情報マネージャー)]
```

## AI エージェント連携（MCP）

Leveret は手で操作するだけでなく、AI エージェントから操作されることも前提に作っています。`Leveret.exe --mcp` は
stdio で動く [Model Context Protocol](https://modelcontextprotocol.io/) サーバーです（ウィンドウを開かず、ネットワークのポートも使いません）。
同じ PC 上の Claude Code、Codex、GitHub Copilot、Claude Desktop、そして Ollama などのローカルモデルを使うものを含む
あらゆる MCP クライアントから、Leveret のすべての機能を使えます。

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/dark/15-agents.png">
  <img alt="AI エージェント連携ダイアログ: 権限、クライアントの設定、ツール、最近の操作" src="docs/screenshots/light/15-agents.png">
</picture>

サイドバーのロボットのアイコン（**AI エージェント連携**）を開くと、exe のパスを埋めた設定がコピーできます。例:

```bat
claude mcp add --scope user leveret -- "C:\Apps\Leveret\Leveret.exe" --mcp
```

```toml
# Codex: ~/.codex/config.toml
[mcp_servers.leveret]
command = 'C:\Apps\Leveret\Leveret.exe'
args = ["--mcp"]
```

| 権限 | エージェントにできること |
|---|---|
| オフ | なし |
| 読み取りのみ | 状態・差分・履歴・ブランチ・スタッシュ・Issue の参照 |
| **標準**（既定） | さらにコミット、プッシュ / プル / フェッチ、ブランチ、マージ、リベース、リバート、チェリーピック、スタッシュ、Issue の作成 / 編集 / コメント / GitHub への反映 |
| すべて | さらに変更の破棄、ブランチの削除、操作の中止 / スキップ、スタッシュの削除、ローカル Issue の削除 |

- **49 個のツール**: JSON スキーマと MCP のアノテーション（読み取り専用 / 破壊的 / 外部通信）付きで、モデルが使いやすい作りです。
  リポジトリは名前でもパスでも（その中の作業ディレクトリでも）指定でき、ブランチは `main` や `origin/main`、リビジョンは `HEAD~1`
  のように書けます。結果は簡潔な JSON で、次にすべきこと（リベースが止まったときの競合ファイルなど）も返します。
- **ユーザーから見える**: エージェントの変更はすべて記録され、アプリに通知が出て表示も自動で更新されます。
- **ツールは1か所** （[`backend/src/agent/tools/`](backend/src/agent/tools/)）: MCP サーバーと Leveret 内で動くエージェント（`agent_call`）が
  共有し、アプリ自身のコマンドと同じ検証を通ります。

## はじめかた

### 必要なもの

- Windows 10 / 11 と WebView2 Runtime（Windows 11 には標準で入っています）
- [Git for Windows](https://gitforwindows.org/)（`PATH` に `git` があること）
- 開発する場合: [Node.js](https://nodejs.org/) 20 以上、[Rust](https://rustup.rs/)（MSVC ツールチェーン）

### ソースから起動

```bat
scripts\setup.ps1     :: ツールの確認、依存パッケージの取得、git フックの有効化
scripts\dev.bat       :: ホットリロードで起動（-Mock でダミーデータ）
```

画面だけ見たいときは `scripts\preview.bat`。Rust をビルドせず、ブラウザでダミーデータを表示します。

### リリースビルド

```bat
scripts\publish.bat   :: -> artifacts\<バージョン>\Leveret.exe と Leveret_v<バージョン>.zip
```

### GitHub サインイン

**Personal Access Token** でのサインインはそのまま使えます。
**ブラウザでのサインイン**（GitHub Desktop と同じ OAuth デバイスフロー）には OAuth App が必要です。

1. GitHub → Settings → Developer settings → **OAuth Apps** → New OAuth App
   （Homepage / Callback URL は任意。例: `https://github.com/<あなた>/Leveret`）
2. **Enable Device Flow** にチェックを入れ、**Client ID** をコピーします（Client Secret は不要です）。
3. `LEVERET_GITHUB_CLIENT_ID` として渡します。ビルド時（`set LEVERET_GITHUB_CLIENT_ID=... && scripts\publish.bat`）でも、
   実行時の環境変数でも構いません。

トークンは Windows 資格情報マネージャーに保存され、git には環境変数経由でのみ渡されます。
コマンドラインやリポジトリの設定ファイルに残ることはありません。

## キーボードショートカット

| キー | 操作 |
|---|---|
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | 変更 / 履歴 / Issue |
| `Ctrl+Enter` | コミット |
| `Ctrl+B` | ブランチ |
| `Ctrl+Shift+N` | 新しいブランチ |
| `Ctrl+Shift+F` / `Ctrl+Shift+L` / `Ctrl+Shift+P` | フェッチ / プル / プッシュ |
| `Ctrl+F` | 履歴を検索 |
| `F5` | 最新の状態に更新 |
| `Shift+F10`、アプリケーションキー、右クリック | コンテキストメニュー |
| `Ctrl+Shift+S` | 画面を保存 |
| `Ctrl+Shift+D` | パフォーマンスパネル |
| `Ctrl+,` | 設定 |

## スクリプト

`.bat` はターミナルからでもダブルクリックでも実行でき、引数は対応する `.ps1` に渡されます。

| スクリプト | 内容 |
|---|---|
| `scripts\setup.ps1` | ツールの確認、依存パッケージの取得、git フックの有効化 |
| `scripts\dev.bat [-Mock]` | ホットリロードで起動 |
| `scripts\preview.bat [-Scene <id>] [-Theme light\|dark] [-App]` | 画面だけ確認（ブラウザ + ダミーデータ。`-App` でデスクトップアプリとして表示） |
| `scripts\test.bat [-Fast]` | セキュリティの静的チェック、`svelte-check`、フロントのビルド、`cargo test`、`cargo clippy` |
| `scripts\security.bat [-Offline]` | セキュリティの静的チェック、`npm audit`、`cargo audit`、バックエンドのセキュリティテスト |
| `scripts\publish.bat [-SkipTests]` | リリースビルド。`artifacts\<バージョン>\` に `Leveret.exe` と `Leveret_v<バージョン>.zip` を出力 |
| `scripts\benchmark.bat [-Runs N] [-Repo <パス>] [-Mock] [-SaveBaseline] [-Strict]` | ベンチマーク。生成した大きなリポジトリで全画面の処理時間を計測し、基準値と比較 |
| `scripts\screenshot.bat [-Browser] [-Theme light\|dark\|both] [-Real] [-Scenes a,b] [-Size 900x560] [-Open]` | 全画面を `artifacts\screenshots\` に撮影（`-Size` は `-Browser` 時のウィンドウサイズ） |
| `scripts\version.bat [<x.y.z> \| major \| minor \| patch]` | バージョンを一括変更（引数なしなら入力を求めます） |
| `scripts\licenses.bat [-Check]` | 設定画面に表示するサードパーティのライセンス一覧を生成（依存パッケージを変えたら実行） |
| `scripts\clean.ps1 [-All]` | ビルド成果物を削除 |

## 撮影モード

アプリはウィンドウを表示せずに自分自身を撮影できます。

```bat
leveret.exe --capture out --mock --theme dark --scenes 02-changes,04-history
```

画面外で描画し（タスクバーにも出ず、フォーカスも奪いません）、
[`frontend/src/lib/capture.ts`](frontend/src/lib/capture.ts) のシーンを順に表示して `out\<シーン>.png` に保存し、終了します。
使用中は **Ctrl+Shift+S** で表示中の画面を `ピクチャ\Leveret\` に保存できます。

## パフォーマンス

速さは推測ではなく計測で確かめます。git のプロセス、GitHub への通信、バックエンドとの往復、画面遷移（描画完了まで）、
プッシュなどの操作、メインスレッドの長い処理、起動時間をすべて計測しています。

- **Ctrl+Shift+D** でパフォーマンスパネルを表示します（処理ごとの中央値 / p95 / 最大。目安を超えたものは赤字）。
- `scripts\benchmark.bat` は大きなリポジトリ（3000 コミット、1200 ファイル、マージ、タグ、変更 200 ファイル）を生成し、
  画面外で全画面を操作して（`leveret.exe --bench`）、基準値と比較します。
- `scripts\dev.bat` で起動すると、すべての計測値を `%LOCALAPPDATA%\com.leveret.desktop\logs\perf.jsonl` に記録します。

## セキュリティ

Leveret は自分自身の WebView を信頼せず、Rust のバックエンドを信頼の境界としています。

- **ブラウザ機能なし**: 開発者ツール、再読み込み、印刷、検索、ソース表示、ズーム、右クリックメニュー、
  戻る / 進む、アプリ外への移動、新しいウィンドウ、ダウンロードはすべて無効です
  （[`lockdown.rs`](backend/src/lockdown.rs)、[`lockdown.ts`](frontend/src/lib/lockdown.ts)）。
- **コマンドの検証**: 登録済みリポジトリのみ、パストラバーサル禁止、git へのオプション注入禁止、
  クローン URL は https / ssh のみ（[`security.rs`](backend/src/security.rs)）。
- **最小限の公開範囲**: 厳格な CSP、プロトタイプの凍結、WebView に許可する権限は4つだけ、shell / fs / http プラグインなし。
  ネットワーク通信はすべて Rust 側で行います。
- **エージェントにも手綱**: MCP は stdio のみ（待ち受けポートなし）、呼び出しごとに権限を確認、破壊的なツールは「すべて」のときだけ、
  変更はすべて記録してアプリに表示します。
- **継続的なチェック**: `scripts\security.bat` で静的ルール、依存関係の監査、セキュリティテストを実行します。

## 信頼性

- **git の操作は1つずつ**: 実行中はボタンやショートカットが無効になるので、ダブルクリックでコマンドが重なることはありません。
- **黙って消えない**: 破棄したファイルはごみ箱へ移動し、止まった操作はいつでも中止できます。リポジトリ一覧は安全に書き込まれ、
  読めなくなった一覧は `repos.json.bak` として残します。
- **障害を閉じ込める**: 画面の表示でエラーが起きても真っ白にはならず「表示し直す」で復帰します。バックエンドのコマンド内で
  panic が起きてもそのコマンドが失敗するだけで、内容は `%LOCALAPPDATA%\com.leveret.desktop\logs\crash.log` に記録されます。

## フォルダ構成

```text
frontend/                 Svelte の UI（package.json、vite.config.ts、tsconfig はここ）
  src/lib/state.svelte.ts   app ストア: リポジトリ、状態、操作を1つずつ実行するガード
  src/lib/changes.svelte.ts 変更タブ、コミット欄、スタッシュ（ほかに history / issues / branches / agents ストア）
  src/lib/github.svelte.ts  GitHub アカウント、サインイン、クローンの状態
  src/lib/api.ts            バックエンド呼び出しの型付きラッパー（往復時間を計測）
  src/lib/perf.ts           計測値、目安、パフォーマンスパネルのデータ
  src/lib/bench.ts          --bench のベンチマーク手順
  src/lib/mock/             ブラウザ / --mock 用のダミーバックエンド
  src/lib/capture.ts        撮影シーンの定義
  src/lib/lockdown.ts       ブラウザのショートカットや操作を無効化
  src/lib/layout.svelte.ts  幅を変えられる領域のサイズ
  src/lib/menu.svelte.ts    アプリ独自の右クリックメニュー
  src/lib/i18n/             メッセージカタログ（locales/ja.ts が領域ごとの locales/ja/*.ts をまとめ、キーを定義）
backend/                  Rust / Tauri（Cargo.toml、tauri.conf.json はここ）
  src/lib.rs                アプリの初期化とコマンドの登録
  src/commands/             Tauri コマンド。領域ごとのモジュール（入力の検証はここ）
  src/git/                  status、diff、log + グラフ、commit、clone、remote、branch、
                            operation（マージ / リベース / リバート）、discard、stash
  src/agent/                AI エージェント用ツール（tools/、領域ごとのモジュール）と MCP サーバー（mcp.rs）
  src/issues/               ローカルの Issue（.git/leveret/issues.json）と GitHub の Issue
  src/perf.rs               計測値の記録、perf.jsonl
  src/settings.rs           settings.json（エージェントの権限）
  src/github.rs             デバイスフロー、トークン保存、リポジトリ一覧
  src/security.rs           WebView から来る入力の検証
  src/lockdown.rs           WebView2 の封じ込め設定
  src/capture.rs            WebView2 による画面撮影、--capture / --bench の引数
  src/crash.rs              クラッシュログと致命的エラーの表示
  icons/generate_icon.py    icons/leveret.svg（アプリアイコン）を描くスクリプト
scripts/                  開発用スクリプト
docs/screenshots/         README で使う画像
```

## 言語の追加

1. `frontend/src/lib/i18n/locales/ja.ts` と `ja/` フォルダを `en.ts` と `en/` などにコピーして翻訳します（型 `Messages` で全キーがそろっているかチェックされます）。
2. `frontend/src/lib/i18n/index.svelte.ts` の `catalogs` に登録します。

バックエンドのエラーは `{ code, detail }` で届き、`errors.<code>` のメッセージで表示されます。

## ロードマップ

- [x] リポジトリ一覧、変更、コミット
- [x] グラフ付きの履歴とコミット詳細
- [x] GitHub サインインとクローン
- [x] フェッチ / プル / プッシュ / 同期
- [x] ブランチ作成 / 切り替え / 削除、チェックアウト
- [x] リバート、チェリーピック、マージ、リベース、スタッシュ
- [x] AI エージェント向け MCP サーバー
- [x] Issue（GitHub とローカル、GitHub への反映）
- [ ] アプリ内エージェント（Claude、Codex、GitHub Copilot、Ollama）
- [x] WinMerge で差分を開く
- [ ] 英語 UI
- [x] タグ
- [ ] 対話的リベース

## コントリビュート

規約は [AGENTS.md](AGENTS.md) を参照してください。要点は、コードのコメントとコミットメッセージは英語
（`commit-msg` フックでチェック）、UI の文言はメッセージカタログ経由のみ、コミット前に `scripts\test.bat`、の3つです。

## ライセンス

[MIT](LICENSE) © valneko-twoon2
