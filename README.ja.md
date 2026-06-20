# terrain

[English](README.md) | 日本語

`terrain` は、Markdown のナレッジベース向けの軽量・設定可能・ローカルファーストな全文検索エンジンです。日本語テキストにも標準で対応しています。

コマンドラインの MCP (Model Context Protocol) サーバーとして動作し、指定したディレクトリ内の `.md` ファイルをインデックス化して、検索・取得用のツールを公開します。

## 特徴

- **全文検索:** `tantivy` を基盤とする `traverze` 検索エンジンを利用。
- **日本語対応:** IPADIC 辞書を用いた `lindera` により、日本語テキストを高精度に形態素解析・トークン化。
- **MCP サーバー:** 標準入出力上にシンプルで機械可読なツールインターフェースを公開。
- **自動インデックス:** 対象ディレクトリを監視し、ファイルの追加・変更・削除・リネームに応じてインデックスを自動更新（再起動は不要）。イベントはデバウンスされ、バッチ処理によって効率的に反映されます。
- **安全性:** `read_file` はインデックスに登録済みのパスのみを返します。すなわち、インデックスへの登録がアクセス許可そのものになります。
- **設定可能:** TOML 設定ファイルでツールの説明文をカスタマイズし、AI モデルの挙動を調整可能。
- **クロスプラットフォーム:** Rust 製で、Windows・macOS・Linux で動作。

## インストール

[Rust](https://www.rust-lang.org/tools/install) がインストールされている必要があります。

### CLI ツールとして

```bash
cargo install terrain
```

### ライブラリとして

`Cargo.toml` に以下を追加します。

```toml
[dependencies]
terrain = { version = "0.2.0", default-features = false }
```

デフォルトフィーチャーを無効にすると、CLI が使用する `clap` と `notify` への依存が外れ、組み込みに適した軽量なライブラリになります。

ライブラリは以下の公開 API を提供します。

- `Config` — TOML 設定ファイルの読み込みとパース。
- `TerrainServer` — `rmcp` のトランスポートに組み込める MCP サーバーハンドラ。`TerrainServer::new(engine, indexed_paths, &config)` で構築します。
- `IndexedPaths` — 現在インデックスに登録されているパスを保持する、クローン可能で共有可能な集合。`read_file` はこの集合を参照して読み取りを認可するため、組み込みアプリ側はパスを登録することでアクセスを制御します。
- `resolve_dir` / `build_engine` — ディレクトリ解決と検索エンジン初期化のためのユーティリティ関数。

ライブラリ自体はディレクトリの走査やファイルシステムの監視を行いません。どのファイルをいつ登録・再インデックスするかは組み込みアプリが決定します。`.md` ファイルのディレクトリを走査し、[`notify`](https://crates.io/crates/notify) でインデックスを同期し続ける統合例については [src/main.rs](src/main.rs) を参照してください。

## MCP クライアントの設定

Claude Desktop などの MCP 対応クライアントで `terrain` を使うには、クライアントの設定ファイル（例: `claude_desktop_config.json`）に以下を追加します。

```json
{
  "mcpServers": {
    "terrain": {
      "command": "terrain",
      "args": ["--dir", "/path/to/your/notes"]
    }
  }
}
```

`cargo install` を使わずにソースからビルドした場合は、代わりに実行ファイルへのフルパス（例: `"/path/to/terrain"`）を指定してください。

## 使い方

1.  **サーバーの起動:**
    Markdown ファイルを含むディレクトリを指定して、ターミナルからプログラムを実行します。

    ```bash
    terrain --dir /path/to/your/notes
    ```

2.  **インデックス化:**
    サーバーはまず、指定ディレクトリ内のすべての Markdown ファイルをインデックス化します。何件のファイルがインデックスされたかを示すメッセージが表示されます。

    ```
    indexed 1234 markdown files from /path/to/your/notes
    ```

3.  **変更の監視:**
    初回インデックス後、サーバーはディレクトリを監視し、インデックスを自動的に同期し続けます。Markdown ファイルの追加・編集・削除・リネーム時に再起動する必要はありません。ファイルシステムのイベントはデバウンスされてバッチ処理され、インデックス更新時には以下のようなログが表示されます。

    ```
    watching /path/to/your/notes for changes
    watcher: re-indexed 1 file(s)
    watcher: removed 1 file(s) from index
    ```

4.  **MCP 経由での操作:**
    インデックス化が完了すると、サーバーは `stdin` で MCP の JSON リクエストを待ち受け、`stdout` にレスポンスを返します。このインターフェースは任意の MCP 対応クライアントやコントローラから利用できます。

## 設定

MCP では、ツールの説明文が「AI モデルがいつ・どのようにそのツールを使うか」の判断に直接影響します。TOML 設定ファイルを指定することで、これらの説明文をユースケースに合わせてカスタマイズできます。

```bash
terrain --dir /path/to/your/notes --config terrain.config.toml
```

利用可能なすべてのオプションについては [terrain.config.example.toml](terrain.config.example.toml) を参照してください。

## MCP ツール

サーバーは以下のツールを提供します。

### `search`

インデックス済みの Markdown ファイルを検索し、該当するファイルパス・スコア・スニペットを返します。

- **説明:** 日本語テキストに高度に最適化されています。ユーザーの質問に答えるための関連コンテキストを見つけるために使用します。該当する絶対ファイルパス、関連度スコア、周辺テキストのスニペットの一覧を返します。
- **パラメータ:**
    - `query` (string, 必須): 検索クエリ。スペース区切りで複数のキーワードを指定できます。
    - `limit` (integer, 任意): 返す検索結果の最大件数（デフォルト: 20）。
- **戻り値の例:**
    ```json
    [
      {
        "path": "/path/to/your/notes/example.md",
        "score": 18.72,
        "snippet": "This is a snippet of text surrounding the matched keyword."
      }
    ]
    ```

### `read_file`

指定した Markdown ファイルの全内容を読み取ります。

- **説明:** `search` ツールで有望なスニペットを見つけ、より詳細なコンテキストが必要な場合に使用します。検索結果から取得した正確な絶対ファイルパスを指定してください。
- **パラメータ:**
    - `path` (string, 必須): 読み取る Markdown ファイルの絶対パス。`search` ツールが返した正確なパスを使用する必要があります。
- **戻り値の例:**
    指定した Markdown ファイルの生の全内容。

## ライセンス

以下のいずれかのライセンスで提供されます。

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

どちらを選択しても構いません。
