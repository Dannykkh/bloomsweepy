# BroomSweepy

<p align="center">
  <img src="apps/desktop/src-tauri/icons/app-icon-master.png" width="112" alt="BroomSweepyのほうきアイコン">
</p>

<p align="center">
  <a href="README.en.md">English</a> |
  <a href="README.md">한국어</a> |
  <strong>日本語</strong> |
  <a href="README.zh-CN.md">简体中文</a>
</p>

BroomSweepyは、WindowsとmacOS向けのストレージ分析・クリーンアップ確認ツールです。**Rust + Tauri 2 + Reactがメインプロジェクト**となり、従来のSwiftUIアプリの広々としたカード、分かりやすいアイコン、ガラス調のUIを受け継ぎます。`BroomSweepy/`のSwiftソースは従来版の参考実装です。

大容量ファイル、内容を検証した重複ファイル、文書検索はローカルのRustエンジンで処理します。AI接続は任意です。1つのアプリで英語・韓国語・日本語・簡体字中国語に対応し、初回は英語で表示します。`Settings > Display language`で変更できます。

## 基本的な使い方

1. ダッシュボードでドライブを選ぶか、`ストレージ整理`でローカルフォルダを選びます。フォルダを選ぶと容量マップが作成されます。
2. 大きい長方形から確認し、フォルダをクリックして下の階層へ進みます。項目メニューから場所を表示したり、個別ファイルのゴミ箱移動を確認したりできます。
3. 大容量ファイルと重複の検査を一度に実行し、対象を自分で選択します。実際の移動にはアプリ内の最終確認が必要です。
4. `パフォーマンス`でCPUとメモリ、`ファイル管理`で名前と本文、`AIアシスタント`で自然言語の質問を扱います。

## 画面プレビュー

現在のRustアプリと同じReactコンポーネントを、公開用のサンプルデータで描画した画像です。ドライブ、ファイル、数値、会話は例であり、実際のAI応答や利用者のファイルではありません。ブラウザーの画像はmacOSのネイティブなウィンドウ材質を再現していません。

### 複数ドライブのダッシュボード

![複数ドライブのダッシュボード](docs/assets/screenshots/v1.6.0-dashboard-ja.png)

### ストレージツリーマップ

![ストレージツリーマップ](docs/assets/screenshots/v1.6.0-overview-ja.png)

### CPUとメモリ

![CPUとメモリ](docs/assets/screenshots/v1.6.0-performance-ja.png)

### AIアシスタント

![AIアシスタント](docs/assets/screenshots/v1.6.0-assistant-ja.png)

### 設定と言語

![設定と言語](docs/assets/screenshots/v1.6.0-settings-ja.png)

## v1.6.0の主な更新

- ガラス調のダークUI、大きな操作ボタン、分かりやすいアイコンと整理されたナビゲーション。
- 選択したドライブを大きなカード、他を小さなカードで表示し、選択時に位置とサイズをアニメーションで交換します。
- CPU・メモリの円形グラフ、上位アプリの使用量、測定間の滑らかな遷移とモーション軽減への対応。
- macOSのアプリメモリ整理はBroomSweepy本体の未使用allocatorページだけをOSに返します。返却量0も正常な完了です。
- ツリーマップと個別ファイル操作、ファイルの同一性・パスの再検証、ゴミ箱操作の記録。
- AI CLIの未導入・実行不可・非互換・ログイン必要を区別し、会話、キャンセル、保存履歴の処理を改善しました。

## 安全性とクラウドの除外

スキャンはファイルを変更しません。大容量・重複検査、ドライブ集計、ツリーマップ、ファイル一覧、文書索引は既知のクラウド同期ルートとオンライン専用項目を除外します。macOSの`~/Library/CloudStorage`、`~/Library/Mobile Documents`と認識済みのGoogle Drive・iCloud・OneDrive・Dropboxパスは走査前に除外し、直接選択しても検査しません。認識済みルート内のダウンロード済みファイルも除外します。任意の場所へ移動した同期フォルダや、すべてのプロバイダーの自動識別は保証しません。

重複はサイズ、部分・全体BLAKE3、最終バイト比較で検証します。ファイル移動は選択、再検証、最終確認、記録を経てOSのゴミ箱を利用します。空フォルダ探索は読み取り専用です。一般的な完全削除、ゴミ箱を空にする操作、レジストリの自動削除はありません。移動した論理サイズは増えた空き容量と同じではありません。

メモリ整理はシステム全体、他のアプリ、WebView補助プロセス、スワップ、メモリリークを解消しません。CPU整理機能はありません。macOSの通常終了要求は別の確認操作で、強制終了には切り替えません。Windowsのパフォーマンスは読み取り専用で、スワップはページファイルの現在使用量ではなくコミット量ベースの推定値です。

Docker管理は初期状態で無効です。有効時も固定コマンドだけを使い、ボリュームを除外し、復元不可の操作として別途確認します。

## AI・CLI・MCP

Codexデスクトップアプリを入れただけでは、Codex CLIが導入されているとは限りません。利用するCLIの導入、互換バージョン、ログイン状態をアプリで確認してください。今回のMac会話フローはCodexで検証しました。Claude Code・Grok・Antigravity・Ollamaのアダプターもありますが、今回すべての実動作を検証したわけではありません。

アプリ内チャットは項目名とサイズを含む上限付きのフォルダ要約、質問、会話履歴を選択したプロバイダーへ送ります。ローカルCLIでもモデル処理がオフラインとは限りません。MCP整理ツールは匿名候補IDと制限付き要約を提供し、承認・削除実行ツールは持ちません。ファイル・文書検索を別途許可すると、パスや一致部分が外部クライアントへ渡る場合があります。最終操作はアプリで確認します。

## 対応環境と開発

Rust stable、Node.js 22+、npmが必要です。WindowsではWebView2とMSVC Build Tools、macOSではXcode Command Line Toolsも必要です。

- `apps/desktop/`: メインのTauri/Reactアプリ
- `crates/bloomsweepy-core/`: 共通Rust分析エンジン
- `crates/bloomsweepy-control/`, `apps/bloomsweepy-mcp/`: ローカル制御規格とCLI/MCPブリッジ
- `BroomSweepy/`: 従来のSwiftUI参考実装

今回の変更はApple Silicon Macでビルド・インストールし、ローカルファイル検査とCodex会話フローを確認しました。最新のWindows実行検証は別途必要で、インストーラーはWindows CIで作成します。Mac検証ビルドはad-hoc署名で、Appleの公証済みではありません。配布ファイルと注意事項は[リリース](https://github.com/Dannykkh/bloomsweepy/releases)で確認してください。

```sh
cd apps/desktop
npm ci
npm run tauri dev
```

```sh
# Repository root
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/desktop
npm run check
npm run test:all
npm run build
npm run tauri build
```

## ドキュメント

[変更履歴](CHANGELOG.md) · [CLI接続と制御](docs/cli-control.md) · [パフォーマンスとメモリの範囲](docs/architecture/startup-memory-status.md) · [安全なゴミ箱操作](docs/architecture/safe-trash-actions.md) · [文書検索](docs/architecture/document-search.md) · [ファイル検索](docs/architecture/fast-file-search.md) · [デザイン](DESIGN.md) · [画面の再現](docs/assets/screenshots/README.md)

## 重要：データ損失と復元に関する責任

BroomSweepyは、利用者が選択して最終確認した項目だけを処理するよう設計されています。ただし、復元の可否はOS権限、ゴミ箱設定、同期サービス、外付け・ネットワークドライブの状態に左右されます。DockerクリーンアップはOSのゴミ箱を使用せず、完了した処理は元に戻せません。

実行前に重要なデータをバックアップし、選択したパス、ファイル、Docker分類を必ず確認してください。法律上免責できない場合を除き、利用者が実行したファイル移動・削除・ゴミ箱を空にする操作・Dockerクリーンアップによるデータ損失や復元失敗について、プロジェクト提供者および貢献者は責任を負いません。
