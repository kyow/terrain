# 変更履歴

このプロジェクトに対するすべての重要な変更はこのファイルに記録されます。

このフォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に基づいており、
このプロジェクトは [Semantic Versioning](https://semver.org/lang/ja/spec/v2.0.0.html) に準拠しています。

## [Unreleased]

## [0.3.0] - 2026-08-11

### Added

- 検索エンジンからツール契約を切り離すための `KnowledgeProvider` トレイトと、terrain が所有する契約型（`SearchHit` / `SearchOptions` / `FileContent`）を追加
- `traverze` をバックエンドとする同梱リファレンス実装 `TraverzeProvider` を、新しい `bundled-provider` フィーチャー配下に追加（`cli` 経由で既定有効）
- 任意の `AsyncRead + AsyncWrite` トランスポート（stdio・名前付きパイプ・Unix ドメインソケット）で給仕する `serve_io` ヘルパーを追加
- 組み込みアプリが `rmcp` に直接依存せずトランスポートを構築できるよう、`rmcp` を再エクスポート（`pub use rmcp`）
- Streamable HTTP トランスポートを追加: `--transport http` で MCP を `/mcp` に HTTP 配信。`--port` と `--host` で bind アドレスを制御（`--host` を値なしで指定すると `0.0.0.0` に bind し、他のマシンからアクセス可能）
- 自前の HTTP サーバー（`axum`/`hyper` など）に組み込める `rmcp` の Streamable HTTP tower `Service` を構築する `streamable_http_service` ヘルパーと `streamable-http` フィーチャーを追加
- 埋め込みホストが自身の name/version で名乗れるよう、MCP `serverInfo` の name/version を上書きする config の `[server]` テーブルを追加
- ツール呼び出しごとの入力（引数）と結果をハンドラ層で観測する `ToolCallObserver` トレイト・`ToolCallEvent`・`TerrainServer::with_observer` を追加。組み込みホストが MCP 入出力を自前の UI に表示できるように。フックはトランスポート（stdio・プロセス内ストリーム・Streamable HTTP）を問わず発火
- インデックス済み全ファイルの絶対パスをソート済みで一覧する `list_files` MCP ツールを追加。`limit` / `offset` でページングでき（`limit: 0` で総件数のみ取得）、検索キーワードが思いつかなくても MCP ホストがどんな文書が存在するかを把握できるように。返されたパスはそのまま `read_file` に渡せる。`KnowledgeProvider` トレイトには必須メソッド `list_files` と新しい契約型 `ListOptions` / `FileList` が加わる（外部の provider 実装にとっては破壊的変更）

### Changed

- `TerrainServer` のツールが `traverze` を直接呼ぶ代わりに `KnowledgeProvider` へ委譲するよう変更。`read_file` のアクセス制御も provider 側へ移動（内部リファクタリング、CLI の振る舞いの変更なし）
- `TerrainServer::new` のシグネチャを `(provider, &config)` に変更（旧 `(engine, indexed_paths, &config)`）
- `traverze` を `bundled-provider` フィーチャー配下のオプショナル依存に変更し、組み込みアプリが `traverze` を引き込まずに terrain へ依存できるように。`build_engine` も同フィーチャーで gate
- config のツール説明を、フラットな `search_description` / `read_file_description` キーから、ツールごとの `[tools.<name>]` テーブル（例: `[tools.search] description = "…"`）へ移動。ツールが増えてもカスタマイズを一般化できるように
- デフォルトの `serverInfo` が `rmcp` ではなく terrain 自身の name/version を報告するよう変更
- `rmcp` を 1.7 → 2.2 に更新（terrain 側のソース変更は不要、JSON の wire format も不変）。rmcp 由来の挙動変化として、2.x は stdio 上のパース不能な JSON-RPC 行を黙って無視する（1.x は `Parse error` を返していた）
- `tokio` のバージョン要求を `1.47.1` から `1` に緩和し、他の依存指定と粒度を統一（バージョン指定は下限のため、解決されるバージョンに変更なし）
- `traverze` を 0.2 → 0.3 に更新し、API 変更（ビルダーによる構築、`index_files`/`remove_files` → `index`/`remove` への改名、`search_with_options` の `search` への一本化）に追従。クエリ前処理は `Plain` を明示指定し、0.3 の新しい `Auto` モード（全トークンを AND 結合するため、MCP ホストが投げがちな投機的な複数キーワードクエリがゼロヒットになる）ではなく、0.2 と同じ検索セマンティクス（空白区切りキーワードの OR 結合 + BM25 ランキング）を維持。traverze 由来の改善として、クエリパースが lenient になり、Tantivy の構文文字を含むクエリがエラーにならなくなった
- `search` ツールの description（組み込みデフォルト・設定例・README）に OR + BM25 のセマンティクスを明記し、MCP ホストが同義語や候補キーワードを複数並べる戦略を取れることが伝わるように

### Fixed

- `tests/build_engine.rs` を `bundled-provider` フィーチャーで gate し、`cargo check --tests --no-default-features` が再びコンパイルできるよう修正

## [0.2.2] - 2026-07-08

### Fixed

- `search` が過去の実行で提供していたディレクトリの古い結果を返さないよう修正: `build_engine` がインデックス作成前にインデックスディレクトリをリセットし、CLI は正規化した `--dir` ごとにインデックスディレクトリを分離することで、同時起動したサーバー同士がインデックスを壊し合わないようにした

## [0.2.1] - 2026-06-26

### Changed

- サポートする最低 Rust バージョン（MSRV）を 1.88 に引き上げ
- 依存クレートを最新版に更新: `rmcp` 0.17 → 1.7、`notify` 7 → 8、`toml` 0.8 → 1、`clap` 4.5 → 4.6、`tokio` 1.49 → 1.52、`serde_json` 1.0.149 → 1.0.150

## [0.2.0] - 2026-06-20

### Added

- ディレクトリの変更を監視してインデックスを自動更新するファイルウォッチャーを追加
- デバウンス処理によるイベントのバッチ処理で効率的なインデックス更新を実現
- ファイルウォッチャーでリネームイベントの正規化に対応
- インデックスに登録済みのパス集合をライブラリと組み込みアプリで共有するための `IndexedPaths` 型を追加

### Changed

- `read_file` のアクセス制御を「固定の base_dir 配下」から「インデックスに登録されているパス」へ変更
- `TerrainServer::new` のシグネチャを `(engine, indexed_paths, &config)` に簡素化
- ファイルウォッチャーとディレクトリ走査をライブラリから CLI バイナリへ移動。組み込みアプリは自身でファイル登録を行う
- `notify` 依存を `cli` フィーチャー配下に移動

### Removed

- `collect_markdown_files` と `start_watcher` をライブラリの公開 API から削除

## [0.1.0] - 2026-03-15

### Added

- 全文検索機能を備えた Markdown インデックスサーバー
- `search` と `read_file` ツールを持つ stdio MCP サーバー
- `clap` によるコマンドライン引数のパース
- TOML ファイルによる MCP サーバー説明文の設定機能
- MCP サーバーの設定ファイル例
- コアロジックをライブラリクレートとして切り出し、依存ライブラリとして利用可能に
- `clap` 依存をオプショナルにする `cli` フィーチャーフラグ
- Apache 2.0 および MIT デュアルライセンス

[unreleased]: https://github.com/kyow/terrain/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/kyow/terrain/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/kyow/terrain/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/kyow/terrain/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kyow/terrain/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kyow/terrain/releases/tag/v0.1.0
