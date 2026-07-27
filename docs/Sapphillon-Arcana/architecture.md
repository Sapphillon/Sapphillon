# Sapphillon Arcana アーキテクチャ設計

> 本ドキュメントはシステムの**構造と設計方針**を定義する。実装詳細（Proto 定義、コマンド仕様、検証ルール、JSON 例、運用フロー詳細）は **[spec.md](./spec.md)** に分離している。セクション間の参照は `§N.N` 形式（architecture.md 内）または `§S.N` 形式（spec.md 内）で示す。

## 目次

- [§1 システムの目的と概要](#1-システムの目的と概要)
- [§2 ツールチェーンとクレート分割アーキテクチャ (Rust)](#2-ツールチェーンとクレート分割アーキテクチャ-rust)
- [§3 スキーマの SSOT 方針](#3-スキーマの-ssot-方針)
- [§4 sapphillon-cli の設計概要](#4-sapphillon-cli-の設計概要)
- [§5 クライアント側の鍵管理と検証モデル](#5-クライアント側の鍵管理と検証モデル)
- [§6 リポジトリシステムと信頼モデル](#6-リポジトリシステムと信頼モデル)
- [§7 リポジトリのアーキテクチャ（PR 運用）](#7-リポジトリのアーキテクチャpr-運用)
- [§8 公式リポジトリ 付加価値バックエンド設計](#8-公式リポジトリ-付加価値バックエンド設計-optional)
- [§9 セキュリティ運用設計](#9-セキュリティ運用設計)
- [§10 システム実装のフェーズ分け](#10-システム実装のフェーズ分け推奨)

---

## 1. システムの目的と概要

**目的:** Sapphillon 向けの拡張プラグインを安全に開発・検証し、分散型リポジトリを通じてエンドユーザーへ配信する一貫したエコシステムを構築する。

**背景と刷新:** 既存の TypeScript 版 CLI を廃止し、Deno 公式の Rust クレート（deno_ast, deno_graph 等）を直接組み込んだ純 Rust 製のツールチェーンに刷新する。これにより、高速かつ厳密な静的解析と、仕様変更に強い安全なバンドルを実現する。

**静的ホスティングを基本とする分散アーキテクチャと公式の付加価値:** 本エコシステムのリポジトリ（インデックス配信）は、**「静的ファイルホスティング」で完全に動作する**ように設計する。これにより、サードパーティは GitHub Pages や Amazon S3 等を用いて、サーバーレスかつ無償・低コストで独自のリポジトリを運用できる。一方で、**公式リポジトリのみ動的 API バックエンドを構築**し、ダウンロード数、スター数、トレンドなどのリアルタイムな統計情報（動的メタデータ）を配信することで、エコシステムの中枢としての付加価値を提供する。

**TUF（The Update Framework）による信頼基盤:** 実体ファイル（`package.js`）は外部 URL にホスティングし、リポジトリインデックスとパッケージの完全性・真正性は、CNCF graduated であり業界標準のセキュリティフレームワークである **TUF** によって多層的に担保する。鍵のローテーション、リプレイ攻撃対策、メタデータ有効期限をすべて TUF 標準機構で実現し、自己完結型の堅牢な信頼モデルを構築する。採用ライブラリは AWS Labs の **`tough` クレート v0.23.0（TUF 1.0.0 準拠）** で、root/timestamp/snapshot/targets/delegated targets の全ロール作成・検証を単体で完結する。

---

## 2. ツールチェーンとクレート分割アーキテクチャ (Rust)

システム全体を Cargo ワークスペースで管理し、責務に応じてライブラリ（クレート）とバイナリを分割する。

### 2.1. コアライブラリ (共有クレート)

**1. sapphillon-arcana (静的解析・メタデータ)**

- **責務:** TS/JS コードのパース、JSDoc からのメタデータ抽出、コードフォーマットおよび要求権限の厳密な静的解析（Validation）。
- **内部技術:** Deno 公式の deno_ast 等を利用し、AST（抽象構文木）レベルで解析を行う。
- **features:**
    - `arcana-loader`: `package.toml` および `package.js` からパッケージ情報の読み込みを行う（完成済み package の読み込み専用）。
    - `arcana-compiler`: ts/js コードのパース、JSDoc からのメタデータ抽出、コードフォーマット、要求権限の解析、および `package.toml` のスキーマ検証（JSON Schema 相当）。

**2. arcana-repo-core (リポジトリドメイン & TUF暗号基盤)**

- **責務:** リポジトリの TUF メタデータ管理、鍵・設定管理 API、およびクライアント・サーバー・ビルダー向けの各種機能提供。
- **内部技術:** `tough` クレートを中核に採用。TUF 仕様の全ロールを取り扱う。
- **features:**
    - `client`: エンドユーザー向け。TUF リポジトリの検証・読み込み（`Repository::load` 等）、データモデル、鍵管理 API。
    - `server`: 公式 API 向け。TUF メタデータの署名・更新機能。
    - `builder`: CLI 向け。TUF メタデータ構築、開発者 delegated targets 署名支援、カスタムメタデータ変換機能。

### 2.2. バイナリツール

- **sapphillon-cli:** プラグイン開発者向けビルド・検証・署名（delegated targets 更新）ツール。
- **arcana-cli:** サードパーティリポジトリ管理者向け TUF インデックス構築・運用ツール。
- **api-server / async-worker:** 公式リポジトリ用動的バックエンドプロセス（TUF 署名・統計情報集計）。

### 2.3. `tough` 依存セキュリティポリシー

`tough` には delegated metadata validation 系の CVE が報告されているため (CVE-2026-6967, CVE-2025-2886 等)、delegated targets を重く使う本設計では以下のセキュリティポリシーを遵守する:

- `tough >= 0.23.0` を必須とする
- `cargo audit` を CI 必須化（全ワークフロー）
- delegated targets / terminating delegation / expired delegated metadata の regression test を自前で保持
- `tough` の minor update 時は TUF compatibility test suite を実行

---

## 3. スキーマの SSOT 方針

エコシステム全体（開発時、実行時、配信時）でデータ構造の矛盾を防ぐため、**Protocol Buffers (Proto3)** を Sapphillon 固有データの真実の単一の情報源（SSOT）とする。

**SSOT の適用範囲（重要）:** TUF 構造メタデータ（root/timestamp/snapshot/targets のロール枠組み）は `tough` クレートの型と TUF 仕様が SSOT であり、Proto では再定義しない（`tough` の署名検証が独自型に依存するため技術的に必須）。Sapphillon が制御するデータ（`TargetCustom` / `PluginManifest` / `PackageToml` / 補助型）のみ Proto SSOT の対象とする。これは HTTP を Proto で再定義しないのと同じで、外部標準の SSOT は外部仕様に委ねる方針である。

**Canonical JSON encoding:** Sapphillon の canonical JSON encoding は **Rust serde の snake_case JSON** とする。protobuf JSON mapping (lowerCamelCase) は使用しない。`prost` 生成型に `#[serde(rename_all = "snake_case")]` を付与し、`Target.custom["sapphillon"]` の canonical 比較は snake_case で行う。

**2層アーキテクチャ:** Proto 生成型は論理スキーマの SSOT だが、実行時検証・JSON Schema 生成は domain model 経由で行う:

```
proto generated types (prost)
  ↓ TryFrom / Into
domain types (serde + schemars + validation)
  ↓
JSON Schema / TUF custom JSON
```

domain model に validation trait を実装し、SemVer 形式、package_id 正規表現、URL HTTPS 制約、tag 最大数等を検証する。パイプラインの詳細ステップは [§S1](./spec.md#s1-ssot-パイプライン詳細) を参照。

**データソース → authoritative source マッピング:**

| データソース | authoritative source | 役割 |
|---|---|---|
| `package.toml` | `PackageConfig` | 開発者が手書きする静的設定 |
| JSDoc / AST | `FunctionDefinition` | 関数定義・permissions・params・returns |
| build 時マージ | `PluginManifest` | package.js に埋め込まれる実行単位 manifest |
| delegated targets | `TargetCustom` | リポジトリ index 上の検索・検証用 metadata |
| repo-policy target | `RepoOverride` | 管理者が付与する lifecycle / policy metadata |

> `permissions` の authoritative source は JSDoc `@permission` のみ。`package.toml` には permissions を書かない（書いても無視）。`PluginManifest` と `TargetCustom` は build 時に JSDoc から生成・同期される。

---

## 4. sapphillon-cli の設計概要

プラグイン開発者向けのビルド・検証・署名ツール。コマンド仕様の詳細は [§S2](./spec.md#s2-sapphillon-cli-コマンド仕様) を参照。

### 4.1. コマンド一覧

| コマンド | 概要 |
|---|---|
| `init` | `package.toml` とエントリポイントのテンプレート生成 |
| `keygen` | プラグイン開発者用 TUF ed25519 鍵ペア生成（delegated targets 署名用） |
| `check` | ソースコードとメタデータの静的解析 |
| `build` | `check` パス後にバンドルし、`package.js` を出力 + delegated targets エントリ生成・署名 |

### 4.2. 静的解析の方針 (check)

`package.toml` の必須フィールド整合性検証、JSDoc 構文検証（`@param`, `@returns`, `@permission`）、および権限 (Permission) 検証を行う。権限の種類は Deno の Permission モデルに準拠し、`read` / `write` / `net` / `run` / `env` / `sys` / `ffi` とする。詳細な構文ルールは [§S3](./spec.md#s3-静的解析仕様) を参照。

**静的解析の限界と runtime sandbox:**

JS/TS の静的解析は本質的に限界がある（dynamic import, `eval`, `Function` constructor, indirect global access 等の完全な検出は困難）。セキュリティモデルは二段構えとする:

1. **build/check 時:** 宣言された permission の抽出・検証 + 危険構文の検出
2. **runtime 時:** Sapphillon Core が capability ベースで permission を強制（isolated realm / worker / sandbox で実行、plugin から直接 OS/FS/Net に触れない、runtime API を freeze）

### 4.3. バンドルアーキテクチャ (build)

従来の「AST 操作による単純な export 削除」は廃止し、**専用のエントリラッパーを動的に生成し、バンドルツールに処理を委ねる安全なアーキテクチャ**を採用する。ビルド時に専用ラッパーをメモリ上に生成し、`deno_graph` / `deno_emit` でバンドル、`PluginManifest` をコメントブロックとして `package.js` に埋め込む。

> `globalThis.__SAPPHILLON_ARCANA_META__` は bundle-time export table 構築用であり、実行時のメタデータ参照ではない。manifest は実行前に package.js から抽出済みのものを使用し、plugin code による上書きは信頼しない。

バンドル手順の詳細と manifest 抽出仕様は [§S4](./spec.md#s4-バンドルアーキテクチャ詳細) を参照。

---

## 5. クライアント側の鍵管理と検証モデル

エンドユーザーの環境（Sapphillon Core）において、サードパーティリポジトリを安全に追加するため、**TUF 標準の信頼モデル** と `arcana-repo-core` の `client` feature API を採用する。

### 5.1. クライアント API 概要

- `ClientConfig`: OS 標準の設定ファイル（`repos.toml`）を読み書きする構造体。
- `fetch_key_info(url)`: 指定 URL から `root.json` を取得し、TUF メタデータ構造の検証を行う（保存はしない）。
- `add_repository(...)`: ユーザー承認後、設定にリポジトリ情報と `root.json` の信頼起点を永続化する。
- `inject_official_repo()`: 公式リポジトリの `root.json` を安全に注入する。公式鍵のローテーションは TUF root ロール機構で処理されるため、バイナリ更新をまたがずに追従可能（→ [§9.1](#91-root-鍵ローテーション運用フロー)）。

### 5.2. パッケージ検証モデル【重要】

TUF 標準の検証チェーンと、Sapphillon 固有の整合性チェックを組み合わせた多層検証を行う:

```
1. root.json        検証（自己署名 + expires）          → 全ロール鍵の信頼起点
2. timestamp.json   検証（timestamp 鍵）+ expires       → リプレイ対策（§6.1 timestamp / §9.1）
3. snapshot.json    検証（snapshot 鍵）                  → 全メタデータ版数一貫性
4. targets.json     検証（targets 鍵）                   → 委譲先発見
5. <author>.json    検証（開発者鍵 = delegated targets） → 開発者署名
6. package.js       DL + hash 照合                        → TUF 検証 + Sapphillon 独自 transport
7. PluginManifest   抽出（/* @arcana-manifest {...} */） → package.js 内
8. 整合性チェック    manifest.functions[*].permissions
                   == custom.function_permissions         → 権限昇格攻撃防御
9. RepoOverlay     repo-policy/overrides.json 検証        → 管理者署名のライフサイクル
10. install policy yanked/deprecated/abandoned 適用        → block/warn/allow 判定
```

ステップ 1-5 は `tough` の標準機能で完結する。ステップ 6 は TUF target metadata に基づく hash/length 検証を行うが、`custom.download_url` からの取得は Sapphillon 独自の `TargetFetcher` が担当する。ステップ 7-8 が Sapphillon 固有の追加検証である。

> **PermissionSet 正規化:** ステップ8 の等価比較は、正規化済み permission set に対して行う。正規化ルール（function_name 昇順ソート、重複排除、domain lowercase + IDNA 正規化等）は [§S5](./spec.md#s5-パッケージ検証フロー詳細) に定義する。

---

## 6. リポジトリシステムと信頼モデル

通信プロトコルは **HTTP (REST/JSON)**。メタデータ構造は **TUF 仕様（v1.0.0）** に準拠し、Sapphillon 固有データは **Protocol Buffers (proto3)** を SSOT として生成した JSON を用いる。

### 6.1. TUF ロール構成と鍵分担

| ロール | ファイル | 署名鍵 | 推奨 TTL | 更新タイミング | 役割 |
|---|---|---|---|---|---|
| **root** | `root.json` | root 鍵（`2-of-3` offline 保持） | 1年 | 再署名: 毎年 / 鍵ローテーション: 2年に1回 | 全ロール鍵の委任・自己署名 |
| **timestamp** | `timestamp.json` | timestamp 鍵（online） | 6〜24時間 | 高頻度（CI cron または API） | 最新性証明 → リプレイ対策 |
| **snapshot** | `snapshot.json` | snapshot 鍵 | 7〜14日 | パッケージ追加/更新時 | 全メタデータの版数一貫性保証 |
| **targets** | `targets.json` | targets 鍵（リポジトリ管理） | 30〜90日 | 委譲追加時 | パス委譲定義（開発者単位） |
| **delegated** | `<author_github_id>.json` | **開発者鍵** | 90〜365日 | 新バージョン登録時 | パッケージ実体のハッシュ・`custom` |

> delegated targets の TTL は開発者鍵の安全性と運用継続性のバランスを取る。timestamp で最新性を担保し、delegated targets は長期間有効とする。

**鍵の所在:**
- root / timestamp / snapshot / targets 鍵: リポジトリ管理者が保持（CI は Actions Secret 経由で署名、root 鍵のみ厳格に offline 保持）。
- 開発者鍵: 各開発者が `sapphillon-cli keygen` で生成・管理。

> **正規化ルール**: TUF 仕様が標準の JSON canonical serialization を定義するため、旧 Minisign 時代の独自正規化ルール（map 禁止・ソート強制等）は廃止。TUF 標準に従う。

**snapshot metadata の方針:** snapshot.json の `meta` フィールドには、targets.json および全 delegated targets の version / length / hashes を含める。これにより delegated metadata のすり替え・キャッシュ汚染を防止する（TUF 標準準拠）。

**consistent snapshot 方針:** `consistent_snapshot: true` を採用する。metadata は versioned filename (例: `42.snapshot.json`) で配信する。静的ホスティング + CDN 環境で古い snapshot と新しい timestamp の組み合わせが見える事故を防ぐためである。`tough` が consistent snapshot を生成・検証できるかは Phase 0 の spike test で確認する。

### 6.2. ターゲットパス規約

TUF はパス文字列でターゲットを一意識別する。Sapphillon のパス規約:

```
packages/<author_github_id>/<package_id>/<version>/package.js
```

- **委譲単位**: `packages/<author_github_id>/*` → 開発者ごとに1つの delegated role
- 設定: `PathSet::Paths(["packages/<author>/*"])`, `threshold: 1`, `terminating: true`
- パスは識別子であり、ダウンロード URL とは別（[§6.6](#66-url-ポリシー) を参照）

> identifier (`author_github_id`, `package_id`, `version`) には厳密な制約を設ける。検証ルールは [§S3](./spec.md#s3-静的解析仕様) に定義する。

> **GitHub identity の安定化:** `author_github_id` (GitHub username) はパス識別子として使用するが、`TargetCustom.github_numeric_id` および `PackageConfig.github_numeric_id` に immutable な GitHub numeric user id を含める。所有権検証は numeric user id で行う。username は変更可能だが numeric id は不変であるため、identity の安定性を担保する。

### 6.3. パッケージの不変性（Immutable）とライフサイクルポリシー

1. **再登録・上書きの禁止:** 一度登録されたバージョン（パス）の `hashes` は不変。
2. **URL の変更:** `custom.download_url` は自由に変更可能（詳細は [§6.6](#66-url-ポリシー)）。
3. **Yank（取り下げ）の扱い:** 物理削除は行わず、該当 target のライフサイクル状態を yanked に設定。クライアント側で新規インストールをブロック。
4. **Deprecated（非推奨）:** パッケージ全体の非推奨フラグ。インストール時に警告表示。

| 属性 | 対象 | 効果 |
|---|---|---|
| `yanked` | バージョン単位 | **新規インストールブロック**、既存は維持 |
| `deprecated` | パッケージ全体 | 新規・既存とも**警告**、インストール可能 |

> **署名責任境界（重要）:** yanked / deprecated などのライフサイクル情報は、管理者が操作できる必要がある。これらを開発者署名の delegated targets 内に置くと、管理者は開発者鍵なしで変更できず運用が破綻する。本設計では `repo-policy/overrides.json` を独立した TUF target として配置し、targets 鍵（管理者鍵）で署名する。これにより管理者はライフサイクル情報を開発者鍵なしで即時変更できる。`repo-policy/overrides.json` は targets.json の直接ターゲット (targets フィールド) として登録し、targets 鍵で署名する。delegated targets には配置しない。詳細は [§S6](./spec.md#s6-sapphillon-固有データスキーマ-repositoryproto) を参照。

### 6.4. Sapphillon 固有データスキーマ

TUF 構造体は `tough` に任せ、Sapphillon 固有データのみ Proto で定義する。完全な Proto 定義（`repository.proto`）は [§S6](./spec.md#s6-sapphillon-固有データスキーマ-repositoryproto) を参照。TUF メタデータの JSON マッピング例は [§S7](./spec.md#s7-tuf-メタデータマッピング例) を参照。

### 6.5. URL ポリシー

TUF では target のハッシュが署名対象のため、**ダウンロード URL は `custom.download_url` で自由に変更可能**。ハッシュが一致する限り（同一 `package.js`）、URL 再配置は自由。CDN 移行・ホスト障害対応も可能。

ただし、`custom.download_url` からの取得は TUF 標準の target fetch ではなく **Sapphillon 独自 transport** である。検証は TUF 標準（hash/length）、URL 解決・取得は Sapphillon 独自と明確に分離する。

download_url にはセキュリティ上の制限（HTTPS のみ、private IP 拒否、redirect 制限等）を課す。詳細は [§S8](./spec.md#s8-url-ポリシー) を参照。

### 6.6. 静的ホスティングでの timestamp 運用

`timestamp.json` は短 TTL のため頻繁再署名が必要:

| リポジトリ種別 | 再署名方式 |
|---|---|
| **公式リポジトリ**（動的バックエンド） | `async-worker` が定時/イベント駆動で再署名 |
| **サードパーティ**（静的ホスティング） | GitHub Actions の `schedule` cron（例: 6時間毎）で `arcana-cli` が再署名して commit |

> timestamp 鍵は online 運用を前提とするため、Actions Secret での保持が許容される。root 鍵のみ厳格に offline 保持。

### 6.7. arcana-cli（サードパーティリポジトリ向け・静的運用）

リポジトリ管理者向けの TUF インデックス構築・運用ツール。コマンド仕様の詳細は [§S9](./spec.md#s9-arcana-cli-コマンド仕様) を参照。

---

## 7. リポジトリのアーキテクチャ（PR 運用）

分散型のリポジトリシステムを採用する。リポジトリは TUF メタデータ群を通じてパッケージの保管場所（URL）、メタデータ、鍵を管理する。

**リポジトリへのパッケージ登録フロー（推奨）:** GitHub Repository に対して Pull Request を送信するワークフローを想定する。GitHub Actions により、開発者署名検証、ハッシュ照合、メタデータ整合性、パス規約・重複チェックが自動検証される。マージ後に CI/CD が snapshot.json と timestamp.json を再署名してリポジトリインデックスを更新する。CI 検証項目の詳細は [§S10](./spec.md#s10-pr-検証項目) を参照。

新規開発者のオンボーディング PR、所有権移転、放棄パッケージの扱いは [§9](#9-セキュリティ運用設計) を参照。

---

## 8. 公式リポジトリ 付加価値バックエンド設計 (Optional)

公式リポジトリは OCI 等の Kubernetes 上で動的バックエンドとしてセルフホストし、統計情報の提供と高頻度な TUF メタデータ更新を行う。

**k8s コンポーネント:**

- **api-server:** 最新の TUF メタデータ群を返却し、DB の動的データ（`DynamicMetadata`: stars/downloads/rating）を付与して返却。`GET /v1/download` でトラッキング後、`custom.download_url` へ HTTP 302 リダイレクト。`DynamicMetadata` は TUF 署名対象外（参考情報）。
- **async-worker:** リポジトリの TUF 署名鍵（root 鍵を除く online 鍵）を安全に管理。Webhook 受信時にメタデータ DB を更新。最新の TUF メタデータを構築・再署名してキャッシュを更新。timestamp.json を高頻度で再署名。

> **エンドポイント分離原則:** TUF 署名対象 metadata と未署名の動的 metadata は API レベルで分離する。クライアントは動的 metadata を install policy に使用してはならない。詳細は [§S11](./spec.md#s11-公式-api-エンドポイント仕様) を参照。

---

## 9. セキュリティ運用設計

### 9.1. root 鍵ローテーション運用フロー

#### 多重鍵しきい値構成（採用）

root ロールを **`threshold: 2` of 3** の多重鍵にする。3つの鍵は **独立した保守者** が **offline**（HSM/エアギャップ/ハードウェアトークン）で保持。これにより:

- 単一鍵の漏洩だけでは攻撃成立せず（署名 quorum 不足）
- 1鍵消失しても残り2鍵でローテーション可能

#### 定期運用

| 項目 | 方針 |
|---|---|
| 有効期限（`expires`） | 1年 |
| root metadata 再署名 | **毎年**（expires 更新） |
| root 鍵ローテーション | **2年に1回**（鍵ペア入れ替え） |
| 移行期間 | 旧 root の `expires` まで（最大1年）。TUF クライアントは新 root 検証で **自動移行**、ユーザー操作不要 |

> TUF root rotation では、新 root.json は旧 root threshold による署名と新 root threshold による署名の両方（クロス署名）を満たす必要がある。

緊急ローテーション（漏洩疑い時）・公告チャネルの詳細フローは [§S12](./spec.md#s12-root-鍵ローテーション詳細フロー) を参照。

### 9.2. 所有権移転

#### 移転の正当性検証

| ケース | 正当性証明 |
|---|---|
| **任意移転**（旧所有者存活） | 旧所有者が PR 上で `/transfer approve @newowner` コメント（GitHub アイデンティティで署名相当） |
| **アカウント削除** | GitHub の username 再利用防止ポリシーを証拠とし、リポジトリ管理者が裁量承認。新所有者の GitHub Org 所属等の裏付けを推奨 |
| **長期放棄** | **移転の根拠とはしない**。警告のみ発する |

> 放棄のみを理由とした移転は **受け付けない**。所有権移転には旧所有者の能動的承認またはアカウント削除の客観的証拠が必須。

移転フロー・放棄パッケージ警告の詳細は [§S13](./spec.md#s13-所有権移転詳細フロー) を参照。

### 9.3. 新規開発者オンボーディング PR 運用

新規開発者は `targets.json` の `delegations` に自身の公開鍵と委譲ロールを追加する PR を提出する。CI により鍵フォーマット検証、path 一致性、重複チェック、identity verification、鍵所有証明が行われる。CI 検証項目の詳細は [§S14](./spec.md#s14-オンボーディング-pr-ci-検証) を参照。

承認ポリシー: 既存リポジトリメンテナ **1名以上** の approval 必須（自動マージ不可）。

### 9.4. timestamp cron 失敗時アラート

timestamp.json の期限切れを多層（L1/L2/L3）で検知する。詳細は [§S15](./spec.md#s15-timestamp-cron-アラート詳細) を参照。

> 監視対象は timestamp.json のみならず、targets.json / 各 delegated targets / snapshot.json の `expires` も含む。実運用では「delegated metadata が期限切れで作者全体が使えない」事故の方が起きやすい。

### 9.5. 開発者鍵の運用

開発者鍵の紛失・漏洩時の運用フローを定義する。詳細は [§S16](./spec.md#s16-開発者鍵の運用) を参照。

- **紛失時:** targets.json から当該 keyid を削除し、新鍵で delegated targets を再署名
- **漏洩時:** 即座に delegation を削除し、必要に応じて RepoOverride で yank
- **複数鍵 / threshold:** delegated role の threshold はデフォルト 1。重要パッケージは threshold > 1 を許可

---

## 10. システム実装のフェーズ分け (推奨)

手戻りを防ぐため、以下の順序での実装を推奨する。

- **Phase 0: Threat Model & Encoding Spec**
    - TUF metadata layout / consistent snapshot の有無の決定
    - `custom.download_url` fetch policy の策定
    - Proto JSON mapping / permission canonicalization の固定
    - `tough` の `_extra` round-trip spike test
    - repo overlay 署名モデルの確定

- **Phase 1: 基礎ドメインと TUF 暗号・解析基盤 (Core Libraries)**
    - Protobuf スキーマ（`repository.proto`、[§S6](./spec.md#s6-sapphillon-固有データスキーマ-repositoryproto)）の定義と Rust コード生成（`prost`）。
    - TOML スキーマ（`schemars`）の自動生成パイプラインの確立。
    - `arcana-repo-core`: `tough` クレート統合による TUF メタデータ検証・署名基盤の構築。
    - `sapphillon-arcana`: AST パース、JSDoc 権限抽出。

- **Phase 2: プラグイン開発者向けツールチェーン (sapphillon-cli)**
    - `init`, `keygen`（TUF ed25519 鍵ペア生成）実装。
    - `check`（静的解析）の組み込み。
    - 仮想エントリポイント生成と `deno_graph`/`deno_emit` による安全なバンドル（`build`）、manifest 埋め込み、delegated targets エントリ生成と開発者署名。

- **Phase 3: 静的リポジトリ管理ツール (arcana-cli)**
    - リポジトリ管理用 `init`, `add-version`, `yank`, `sign-timestamp` 実装。
    - TUF メタデータ全体の構築・再署名を含む `build` コマンドの実装。
    - timestamp 定時再署名用 GitHub Actions cron ワークフローの整備（[§S15](./spec.md#s15-timestamp-cron-アラート詳細) の監視も併設）。

- **Phase 4: クライアントサイドの統合 (Sapphillon Core API)**
    - `arcana-repo-core` `client` feature による TUF 信頼起点登録フロー。
    - [§5.2](#52-パッケージ検証モデル重要) の多層検証パイプラインの実装（TUF チェーン + マニフェスト整合性チェック）。

- **Phase 5: 公式リポジトリの動的バックエンド (k8s / Optional)**
    - Webhook ワーカー（async-worker）による自動署名・TUF インデックス更新。
    - 統計情報とリダイレクトを提供する API サーバー（api-server）の実装。
