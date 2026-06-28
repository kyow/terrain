# 変更履歴

このプロジェクトに対するすべての重要な変更はこのファイルに記録されます。

このフォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に基づいており、
このプロジェクトは [Semantic Versioning](https://semver.org/lang/ja/spec/v2.0.0.html) に準拠しています。

## [Unreleased]

### Added

- 検索エンジンからツール契約を切り離すための `KnowledgeProvider` トレイトと、terrain が所有する契約型（`SearchHit` / `SearchOptions` / `FileContent`）を追加
- `traverze` をバックエンドとする同梱リファレンス実装 `TraverzeProvider` を、新しい `bundled-provider` フィーチャー配下に追加（`cli` 経由で既定有効）
- 任意の `AsyncRead + AsyncWrite` トランスポート（stdio・名前付きパイプ・Unix ドメインソケット）で給仕する `serve_io` ヘルパーを追加
- 組み込みアプリが `rmcp` に直接依存せずトランスポートを構築できるよう、`rmcp` を再エクスポート（`pub use rmcp`）
- Streamable HTTP トランスポートを追加: `--transport http` で MCP を `/mcp` に HTTP 配信。`--port` と `--host` で bind アドレスを制御（`--host` を値なしで指定すると `0.0.0.0` に bind し、他のマシンからアクセス可能）
- 自前の HTTP サーバー（`axum`/`hyper` など）に組み込める `rmcp` の Streamable HTTP tower `Service` を構築する `streamable_http_service` ヘルパーと `streamable-http` フィーチャーを追加

### Changed

- `TerrainServer` のツールが `traverze` を直接呼ぶ代わりに `KnowledgeProvider` へ委譲するよう変更。`read_file` のアクセス制御も provider 側へ移動（内部リファクタリング、CLI の振る舞いの変更なし）
- `TerrainServer::new` のシグネチャを `(provider, &config)` に変更（旧 `(engine, indexed_paths, &config)`）
- `traverze` を `bundled-provider` フィーチャー配下のオプショナル依存に変更し、組み込みアプリが `traverze` を引き込まずに terrain へ依存できるように。`build_engine` も同フィーチャーで gate

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

[unreleased]: https://github.com/kyow/terrain/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/kyow/terrain/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kyow/terrain/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kyow/terrain/releases/tag/v0.1.0
