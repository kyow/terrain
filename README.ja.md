# terrain

[English](README.md) | 日本語

`terrain` は、Markdown のナレッジベース向けの軽量・設定可能・ローカルファーストな全文検索エンジンです。日本語テキストにも標準で対応しています。

コマンドラインの MCP (Model Context Protocol) サーバーとして動作し、指定したディレクトリ内の `.md` ファイルをインデックス化して、検索・取得用のツールを公開します。

## 特徴

- **全文検索:** `tantivy` を基盤とする `traverze` 検索エンジンを利用。
- **日本語対応:** IPADIC 辞書を用いた `lindera` により、日本語テキストを高精度に形態素解析・トークン化。
- **MCP サーバー:** stdio または Streamable HTTP 上に、シンプルで機械可読なツールインターフェースを公開。
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
terrain = { version = "0.3", default-features = false }
```

デフォルトフィーチャーを無効にすると、CLI が使用する依存（`clap`・`notify`・`axum`）と同梱の `traverze` プロバイダが外れ、自前の検索エンジンを持ち込む前提の軽量なライブラリになります。必要に応じてフィーチャー単位で有効化できます。

- `bundled-provider` — リファレンス実装の `TraverzeProvider` と `resolve_dir` / `build_engine`。
- `streamable-http` — `streamable_http_service` ヘルパー（Streamable HTTP トランスポート）。

ライブラリは以下の公開 API を提供します。

- `Config` — TOML 設定ファイルの読み込みとパース。
- `KnowledgeProvider` — `search` / `read_file` / `list_files` ツールを支える trait。`SearchHit`・`SearchOptions`・`FileContent`・`ListOptions`・`FileList` 型を伴います。これを実装することで、独自の検索エンジンとアクセス制御ポリシーを差し込めます。
- `TerrainServer` — `rmcp` のトランスポートに組み込める MCP サーバーハンドラ。`TerrainServer::new(provider, &config)`（`provider` は `Arc<dyn KnowledgeProvider>`）で構築します。
- `ToolCallObserver` / `ToolCallEvent` — ツール呼び出しごとの引数・結果・所要時間をハンドラ層で観測し、組み込みホストが MCP の入出力を自前の UI に表示できるようにします。`TerrainServer::with_observer(observer)` で登録します。フックはどのトランスポートで給仕しても発火し、リクエストパス上で同期的に呼ばれるため、実装はブロックせずイベントを受け渡すだけにしてください。
- `IndexedPaths` — 現在インデックスに登録されているパスを保持する、クローン可能で共有可能な集合。同梱プロバイダはこの集合を参照して `read_file` の読み取りを認可するため、組み込みアプリ側はパスを登録することでアクセスを制御します。
- `serve_io` — 任意の `rmcp` I/O トランスポート（stdio・パイプ・ソケット）上でサーバーを給仕します。
- `streamable_http_service` *(`streamable-http` フィーチャー)* — 自前の HTTP サーバー（`axum`/`hyper` など）に組み込める `rmcp` の Streamable HTTP tower `Service` を構築します。セッション・SSE の挙動と受信時の `Host`/`Origin` 検証を制御する `StreamableHttpServerConfig`（こちらも再エクスポート）を引数に取ります。
- `TraverzeProvider` / `resolve_dir` / `build_engine` *(`bundled-provider` フィーチャー)* — `traverze` を基盤とするリファレンスプロバイダと、ディレクトリ解決・エンジン初期化のためのユーティリティ。`build_engine` は渡されたインデックスディレクトリをリセットしてゼロから再構築するため、インデックスは常に渡したファイルのみを反映します。
- `rmcp` *(再エクスポート)* — terrain が給仕に使っている `rmcp` クレートをそのまま再エクスポートします。組み込みアプリは `rmcp` への独自の依存を追加せずにトランスポートなどの `rmcp` の値を構築でき、バージョンのずれも避けられます。

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

ネットワーク経由で接続するクライアントの場合は、HTTP トランスポートでサーバーを起動し（[トランスポート](#トランスポート)を参照）、コマンドの代わりにエンドポイント URL を指定します。

```json
{
  "mcpServers": {
    "terrain": {
      "url": "http://127.0.0.1:8000/mcp"
    }
  }
}
```

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

## トランスポート

`terrain` は MCP を 2 つのトランスポートで提供し、`--transport` で選択します。

- `stdio`（デフォルト） — 標準入出力で通信します。多くの MCP クライアント（Claude Desktop など）はサーバーをサブプロセスとして起動するため、こちらを使います。
- `http` — [Streamable HTTP トランスポート](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#streamable-http) を `/mcp` で提供し、クライアントがネットワーク経由で接続できます。

### Streamable HTTP

```bash
# 127.0.0.1:8000 で待ち受け（このマシンのみ）
terrain --dir /path/to/your/notes --transport http

# ポートを変更
terrain --dir /path/to/your/notes --transport http --port 9000

# 他のマシンから到達可能にする（0.0.0.0 に bind）
terrain --dir /path/to/your/notes --transport http --host
```

エンドポイントは `http://<host>:<port>/mcp` です。

| フラグ | デフォルト | 説明 |
|------|---------|-------------|
| `--transport <stdio\|http>` | `stdio` | 給仕するトランスポート。 |
| `--port <PORT>` | `8000` | `http` トランスポートのポート。 |
| `--host [ADDR]` | `127.0.0.1` | `http` の bind アドレス。省略でローカルのみ。値なしで付けると `0.0.0.0`（他のマシンから到達可能）。アドレスを渡すと特定のインターフェースに bind。 |

> **セキュリティ:** `terrain` には認証機構がありません。到達範囲は bind アドレスだけで決まります。デフォルト（`127.0.0.1`）ではサーバーはこのマシン内に閉じています。`--host` は信頼できるネットワークでのみ使用し、認証付きの公開が必要な場合はリバースプロキシ・SSH トンネル・VPN などの背後に `terrain` を置いてください。

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
    - `query` (string, 必須): 検索クエリ。スペース区切りで複数のキーワードを指定できます。キーワードは OR で結合され、多くのキーワードにマッチする文書ほど BM25 スコアで上位になります。
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

### `list_files`

インデックス済みの全 Markdown ファイルの絶対パスを、ソート済み・ページング付きで一覧します。

- **説明:** どんなキーワードで検索すべきか分からないときに、どのような文書が存在するかを把握したり、ナレッジベースの全体像を掴んだりするために使用します。返されたパスはそのまま `read_file` ツールに渡せます。
- **パラメータ:**
    - `limit` (integer, 任意): 返すファイルパスの最大件数（デフォルト: 100）。0 を指定すると総件数のみを取得できます。
    - `offset` (integer, 任意): ソート済みリストの先頭からスキップする件数（デフォルト: 0）。`limit` と組み合わせることで、大きなナレッジベースをページングして取得できます。
- **戻り値の例:**
    ```json
    {
      "total": 245,
      "offset": 0,
      "paths": [
        "/path/to/your/notes/example.md",
        "/path/to/your/notes/ideas.md"
      ]
    }
    ```

## ライセンス

以下のいずれかのライセンスで提供されます。

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

どちらを選択しても構いません。
