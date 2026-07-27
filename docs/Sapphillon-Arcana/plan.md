# Sapphillon Arcana — 実装計画 (plan.md)

> 対象: architecture.md + spec.md (commit `5eb84aa` 時点)
> 作成日: 2026-07-07

architecture.md §10 のフェーズ分けを具体的なタスクに落とし込む。各フェーズは完了条件が明確で、次のフェーズに進む前に検証が可能なものとする。

---

## プロジェクト構成

```
sapphillon-arcana/
├── Cargo.toml                  # ワークスペースルート
├── proto/
│   └── repository.proto        # Proto SSOT (spec.md §S6)
├── crates/
│   ├── sapphillon-arcana/      # 静的解析・メタデータ (library)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs       # deno_ast による TS/JS パース
│   │       ├── jsdoc.rs        # JSDoc 抽出 (@param, @returns, @permission)
│   │       ├── permission.rs   # Permission 検証 (Deno モデル)
│   │       ├── validator.rs    # identifier validation, ArcanaValidate
│   │       ├── manifest.rs     # PluginManifest 構築・直列化
│   │       └── bundle.rs       # ラッパー生成 + deno_emit バンドル
│   │
│   └── arcana-repo-core/       # TUF リポジトリ基盤 (library)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── proto.rs        # prost 生成型 + serde + schemars domain model
│           ├── tuf/
│           │   ├── mod.rs
│           │   ├── builder.rs  # TUF メタデータ構築 (delegated targets, repo-policy)
│           │   ├── client.rs   # TUF リポジトリ検証・読み込み
│           │   ├── server.rs   # TUF メタデータ署名・更新
│           │   └── fetcher.rs  # TargetFetcher (URL policy + hash 検証)
│           ├── keys.rs         # ed25519 鍵管理
│           └── config.rs       # ClientConfig, repos.toml
│
├── binaries/
│   ├── sapphillon-cli/         # プラグイン開発者向け CLI
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   │
│   └── arcana-cli/             # リポジトリ管理者向け CLI
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
│
├── spec.md
├── architecture.md
└── design_review/
```

---

## Phase 0: Spike Test & 仕様確定

**目的:** 実装に入る前の技術的不確実性を排除する。コードを書く前に回答を得る。

### 0-1. `tough` `_extra` round-trip spike test

**ゴール:** `tough` で root.json / targets.json の `_extra` フィールド (unknown field) が parse → serialize → sign で保持されるかを確認する。

**手順:**
1. `tough` v0.23.0 を依存に含む最小 Cargo プロジェクトを作成
2. `_extra` フィールド付きの root.json / targets.json を生成
3. `RepositoryEditor` で署名 → ファイル出力
4. `Repository::load` で再読み込み → `_extra` が保持されているか検証

**完了条件:**
- `_extra` が round-trip で保持される → architecture.md §S12 の `_extra` 方針を確定
- 保持されない → 代替案 (out-of-band 公告チャネルのみ) に切り替え

### 0-2. `tough` consistent snapshot spike test

**ゴール:** `tough` が consistent snapshot (`consistent_snapshot: true`) の metadata を生成・検証できるか確認する。

**手順:**
1. consistent snapshot 有効の TUF リポジトリを `tough` で生成
2. versioned filename (例: `42.snapshot.json`) で出力されるか確認
3. `Repository::load` で検証できるか確認

**完了条件:**
- consistent snapshot が動作する → architecture.md §6.1 の方針を確定
- 動作しない → `consistent_snapshot: false` に切り替え、CDN cache purge 手順を策定

### 0-3. `tough` `spec_version` spike test

**ゴール:** `tough` が `spec_version: "1.0.0"` を受け入れるか確認する。

**手順:**
1. `spec_version: "1.0.0"` の metadata を生成
2. `Repository::load` でエラーが出ないか確認

**完了条件:** `"1.0.0"` が受け入れられるか、`"1.0"` に切り替えるかを確定

### 0-4. `tough` direct target in targets.json spike test

**ゴール:** targets.json の `targets` フィールドに直接エントリ (`repo-policy/overrides.json`) を含め、delegated targets と併用できるか確認する。

**手順:**
1. targets.json に direct target と delegated roles の両方を含む metadata を生成
2. `Repository::load` で両方の target が解決できるか確認

**完了条件:**
- 動作する → H-2 (overrides.json 配置方式) を確定
- 動作しない → 管理者専用 delegated role に切り替え

### 0-5. deno_emit バンドル spike test

**ゴール:** `deno_emit` v0.46.0 で最小限の TS ファイルをバンドルし、`package.js` として出力できるか確認する。

**手順:**
1. `deno_graph` でモジュールグラフを構築
2. `deno_emit` で bundle を実行
3. 出力が単一 JS ファイルになるか確認
4. 外部依存 (`npm:`, `http:`) が含まれる場合の挙動を確認

**完了条件:** バンドル方式が確定し、Phase 2 の実装方針が固まる

---

## Phase 1: 基盤構築 (Core Libraries)

**前提:** Phase 0 の spike test が全て完了していること。

### 1-1. Cargo ワークスペース初期化

**作業内容:**
- ワークスペースルート `Cargo.toml` の作成
- クレートディレクトリ構造の作成 (`crates/sapphillon-arcana/`, `crates/arcana-repo-core/`, `binaries/sapphillon-cli/`, `binaries/arcana-cli/`)
- 共通設定 (`rust-version`, `edition`, 共通 dependencies)

**成果物:** `cargo check` が通る空のワークスペース

### 1-2. Proto スキーマ定義とコード生成パイプライン

**作業内容:**
- `proto/repository.proto` の作成 (spec.md §S6 の Proto 定義をそのまま使用)
- `prost` + `prost-build` による Rust コード生成 (`build.rs`)
- `prost` 生成型 → serde `snake_case` JSON 変換の確認
- domain model 型の作成 (`TryFrom` / `Into` による変換)
- `schemars` による JSON Schema 生成の確認

**成果物:**
- `cargo build` で Proto 生成型がコンパイルできる
- `TargetCustom`, `PluginManifest`, `PackageToml`, `RepoOverrideIndex`, `Permission` の domain model が serde JSON に変換できる
- JSON Schema (`schema.json`) が出力できる

### 1-3. `arcana-repo-core` TUF 基盤 (`tough` 統合)

**作業内容:**
- `tough` v0.23.0 の依存追加
- TUF メタデータ生成フローの実装:
  - root.json の生成 (threshold: 2-of-3, 有効期限1年)
  - timestamp.json / snapshot.json の生成と署名
  - targets.json の生成 (direct target: `repo-policy/overrides.json`, delegated roles)
  - delegated targets (`<author>.json`) の生成と署名
- `repo-policy/overrides.json` の生成と targets.json への登録
- 鍵管理 (`ed25519` 鍵ペア生成・永続化・読み込み)

**成果物:**
- TUF メタデータ一式 (root/timestamp/snapshot/targets/delegated) を生成・署名できる
- `RepoOverrideIndex` を JSON に直列化し、target として登録できる
- 再読み込みで `_extra` / consistent snapshot が正しく動作する (Phase 0 結果に依存)

### 1-4. `arcana-repo-core` builder feature

**作業内容:**
- delegated targets へのエントリ追加 API (`add_target`)
- snapshot.json / timestamp.json の再署名 API (`resign_snapshot`, `resign_timestamp`)
- `RepoOverrideIndex` の更新と再署名 API
- `TargetCustom` の JSON 変換と `Target.custom["sapphillon"]` への格納

**成果物:**
- `builder` feature 経由で TUF メタデータの構築・更新ができる
- `cargo test` で TUF メタデータ生成 → 検証の一連フローが通る

### 1-5. `arcana-repo-core` client feature

**作業内容:**
- `Repository::load` による TUF リポジトリ検証
- 検証フロー (spec.md §S5 ステップ1-5) の実装
- `TargetFetcher` trait の定義と URL policy 実装 (spec.md §S8)
  - HTTPS のみ、private IP 拒否、DNS rebinding 対策、redirect 制限
  - development / production の 2 段階ポリシー
- `TargetCustom` の JSON からのデシリアライズ
- `RepoOverrideIndex` の読み込みと適用 (ステップ9-11)

**成果物:**
- Phase 1-3 で生成した TUF メタデータを `Repository::load` で検証できる
- `TargetCustom` と `RepoOverrideIndex` が正しく読み込める
- URL policy が development / production で切り替わる

### 1-6. `sapphillon-arcana` AST パース基盤

**作業内容:**
- `deno_ast` v0.53.2 の依存追加
- TS/JS ファイルのパース (`deno_ast::parse_module`)
- AST ノード走査による JSDoc コメント抽出 (`@param`, `@returns`, `@permission`)
- `Permission` 検証 (spec.md §S3 の Deno Permission モデル)
  - kind: `read/write/net/run/env/sys/ffi`
  - target: kind に応じた形式チェック
- identifier validation (spec.md §S3 の正規表現)

**成果物:**
- TS/JS ファイルから `@permission` を抽出し、検証できる
- `package.toml` の JSON Schema による検証ができる

### Phase 1 検証

- `cargo test --workspace` が全パスする
- Proto → JSON → Proto の往返テスト
- TUF メタデータ生成 → 検証の一連フロー
- Permission 抽出 → 検証のテスト

---

## Phase 2: プラグイン開発者向けツールチェーン (sapphillon-cli)

**前提:** Phase 1 が完了していること。

### 2-1. sapphillon-cli 骨格と `init` コマンド

**作業内容:**
- `clap` による CLI フレームワーク構築
- `init` コマンド: `package.toml` テンプレートとエントリポイントの雛形生成

**成果物:** `sapphillon-cli init my-plugin` でプロジェクトが初期化される

### 2-2. `keygen` コマンド

**作業内容:**
- ed25519 鍵ペア生成 (`ed25519-dalek` または `tough` の鍵 API)
- 鍵の永続化 (`~/.sapphillon/keys/` またはプロジェクトローカル)
- 公開鍵のエクスポート (TUF 形式)

**成果物:** `sapphillon-cli keygen` で TUF ed25519 鍵ペアが生成される

### 2-3. `check` コマンド (静的解析)

**作業内容:**
- Phase 1-6 の `sapphillon-arcana` ライブラリを CLI から呼び出し
- `package.toml` の検証
- JSDoc 構文検証 (`@param`, `@returns`, `@permission`)
- Permission 検証 (Deno モデル)
- 禁止構文の検出 (`eval`, `Function` constructor, dynamic import)
- identifier validation
- エラー出力のフォーマット

**成果物:** `sapphillon-cli check` で静的解析が実行され、エラーが報告される

### 2-4. `build` コマンド — バンドル

**作業内容:**
- エントリラッパーの動的生成 (`globalThis.__SAPPHILLON_ARCANA_META__`)
- `deno_graph` によるモジュールグラフ構築
- `deno_emit` によるバンドル実行
- manifest コメントブロック (`/*! SAPPHILLON_ARCANA_MANIFEST_V1 <base64url> */`) の埋め込み (spec.md §S4)
- `--dev` フラグ対応 (URL policy 緩和、dev マーカー付与)

**成果物:** `sapphillon-cli build` で `package.js` が出力される

### 2-5. `build` コマンド — delegated targets 署名

**作業内容:**
- `package.js` の SHA-256 ハッシュ・長さ計算
- `TargetCustom` の構築 (package_id, version, permissions, download_url, github_numeric_id)
- delegated targets (`<author>.json`) へのエントリ追加
- 開発者鍵による署名
- manifest と `TargetCustom` の Permission 整合性チェック (spec.md §S5 ステップ8)

**成果物:** `sapphillon-cli build` が `package.js` と更新された delegated targets を出力する

### Phase 2 検証

- テスト用 TS プラグインを作成し、`init → check → build` の一連フローが動作する
- 生成された `package.js` から manifest が抽出できる
- delegated targets が有効な TUF メタデータとして検証できる
- `--dev` フラグで URL policy が緩和される

---

## Phase 3: 静的リポジトリ管理ツール (arcana-cli)

**前提:** Phase 1 が完了していること。Phase 2 と並行実行可能。

### 3-1. arcana-cli 骨格と `init` コマンド

**作業内容:**
- `clap` による CLI フレームワーク構築
- `init <name>` コマンド: TUF 鍵ペア群 (root/timestamp/snapshot/targets) とディレクトリ構造の生成
- `repo-policy/overrides.json` の初期生成 (空の `RepoOverrideIndex`)

**成果物:** `arcana-cli init my-repo` でリポジトリ構造が初期化される

### 3-2. `add-version` コマンド

**作業内容:**
- URL から `package.js` をダウンロード (URL policy 適用)
- `package.js` から manifest を抽出・検証
- `TargetCustom` との整合性チェック
- delegated targets へのエントリ追加
- 重複チェック (同一バージョンの別ハッシュを拒否)

**成果物:** `arcana-cli add-version <url>` でパッケージがリポジトリに登録される

### 3-3. `repo-yank` コマンド

**作業内容:**
- `repo-policy/overrides.json` の `PackageOverride` を更新 (yanked, yank_reason)
- `RepoOverrideIndex` の再署名 (targets 鍵)
- snapshot.json / timestamp.json の再署名

**成果物:** `arcana-cli repo-yank packages/alice/cool-plugin/1.2.0/package.js --reason "malware"` で yank される

### 3-4. `sign-timestamp` コマンド

**作業内容:**
- timestamp.json の緊急再署名
- snapshot.json の再署名 (必要に応じて)

**成果物:** `arcana-cli sign-timestamp` で timestamp が再署名される

### 3-5. `build` コマンド

**作業内容:**
- TUF メタデータ全体の整合性検証
- dist/ への出力 (root/timestamp/snapshot/targets/delegated/repo-policy)
- consistent snapshot の versioned filename 対応

**成果物:** `arcana-cli build` で配信可能な TUF メタデータ一式が出力される

### 3-6. GitHub Actions ワークフロー

**作業内容:**
- timestamp 定時再署名 cron ワークフロー (6時間毎)
- PR 検証ワークフロー (spec.md §S10 の5項目)
- 失敗時アラート (L1: Actions 内部通知)

**成果物:** `.github/workflows/` にワークフロー定義が完成する

### Phase 3 検証

- `init → add-version → repo-yank → build` の一連フローが静的ホスティングで動作する
- 生成された TUF メタデータを `tough` の `Repository::load` で検証できる
- GitHub Actions のワークフローが commit で実行される

---

## Phase 4: クライアントサイド統合 (Sapphillon Core API)

**前提:** Phase 1-3 が完了していること。

### 4-1. クライアント API 実装

**作業内容:**
- `ClientConfig` (repos.toml 読み書き)
- `fetch_key_info(url)` — root.json の取得・検証
- `add_repository(...)` — 信頼起点の永続化
- `inject_official_repo()` — 公式リポジトリの注入

**成果物:** `arcana-repo-core` の `client` feature が API として使える

### 4-2. 多層検証パイプライン実装

**作業内容:**
- spec.md §S5 ステップ1-11 の完全実装:
  - ステップ1-5: TUF 標準検証 (`tough`)
  - ステップ6: `TargetFetcher` 経由のダウンロード + hash/length 検証
  - ステップ7: manifest 抽出 (base64url デコード + JSON パース)
  - ステップ8: PermissionSet 正規化 + 整合性チェック
  - ステップ9: `repo-policy/overrides.json` の検証
  - ステップ10-11: RepoOverride 適用 + install policy 判定

**成果物:** Phase 3 で生成したリポジトリをクライアント API で検証・インストールできる

### 4-3. テスト

**作業内容:**
- エンドツーエンドテスト: Phase 2 でビルド → Phase 3 で登録 → Phase 4 で検証・インストール
- URL policy の development / production テスト
- RepoOverride (yanked/deprecated/abandoned) の適用テスト
- Permission 不一致の検出テスト

**成果物:** `cargo test --workspace` が全パスする

---

## Phase 5: 公式リポジトリ バックエンド (Optional)

**前提:** Phase 1-4 が完了していること。

### 5-1. async-worker

**作業内容:**
- Webhook 受信による TUF メタデータ更新
- TUF 署名 (timestamp/snapshot/targets 鍵)
- timestamp.json の高頻度再署名 (数時間 TTL)

### 5-2. api-server

**作業内容:**
- `/metadata/{role}.json` — TUF 署名対象 metadata
- `/v1/packages` — 未署名動的 metadata (stars, downloads, trends)
- `/v1/download` — トラッキング後リダイレクト (302)

---

## 実装順序の依存関係

```
Phase 0 (spike tests)
    ↓
Phase 1 (core libraries)
    ├── 1-1 ワークスペース
    ├── 1-2 Proto
    ├── 1-3 TUF 基盤 ← 1-2 に依存
    ├── 1-4 builder ← 1-3 に依存
    ├── 1-5 client ← 1-3 に依存
    └── 1-6 AST ← 1-2 に依存
    ↓
Phase 2 (sapphillon-cli) ─── Phase 3 (arcana-cli)  ← 並行可能
    ↓                           ↓
Phase 4 (client integration) ← 両方に依存
    ↓
Phase 5 (backend / optional)
```

---

## 各フェーズの予定期間 (目安)

| フェーズ | 内容 | タスク数 |
|---|---|---|
| Phase 0 | Spike test | 5 |
| Phase 1 | Core libraries | 6 |
| Phase 2 | sapphillon-cli | 5 |
| Phase 3 | arcana-cli | 6 |
| Phase 4 | Client integration | 3 |
| Phase 5 | Backend (optional) | 2 |

---

## 開発規約

### コーディング規約
- Rust edition 2021
- `cargo fmt` / `cargo clippy` を CI 必須化
- `cargo audit` を CI 必須化 (architecture.md §2.3)
- public API には rustdoc コメントを必須

### テスト規約
- unit test: 各クレートの `src/` 内
- integration test: 各クレートの `tests/` ディレクトリ
- e2e test: Phase 4-3 のフローをカバー
- TUF regression test: delegated targets / terminating delegation / expired metadata (architecture.md §2.3)

### コミット規約
- conventional commits 形式 (`feat:`, `fix:`, `test:`, `docs:`, `refactor:`)
- 各タスク完了時にコミット
- Phase 完了時にタグ (`v0.1.0-phase0`, `v0.2.0-phase1`, etc.)

### ブランチ戦略
- `main`: 安定版
- `dev`: 開発統合
- `feat/*`: 各フェーズの作業ブランチ
