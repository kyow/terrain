# 変更履歴

このプロジェクトに対するすべての重要な変更はこのファイルに記録されます。

このフォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に基づいており、
このプロジェクトは [Semantic Versioning](https://semver.org/lang/ja/spec/v2.0.0.html) に準拠しています。

## [Unreleased]

### Added

- ディレクトリの変更を監視してインデックスを自動更新するファイルウォッチャーを追加
- デバウンス処理によるイベントのバッチ処理で効率的なインデックス更新を実現
- ファイルウォッチャーでリネームイベントの正規化に対応
- インデックス完了時にインデックス済みドキュメント数をログ出力する機能を追加

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

[unreleased]: https://github.com/kyow/terrain/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kyow/terrain/releases/tag/v0.1.0
