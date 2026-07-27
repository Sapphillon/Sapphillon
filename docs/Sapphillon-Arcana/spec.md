# Sapphillon Arcana 実装仕様書 (spec.md)

> 本ドキュメントは [architecture.md](./architecture.md) から分離された**実装詳細**を定義する。構造と設計方針は architecture.md を参照すること。セクション参照は `§S<N>` 形式。

## 目次

- [§S1 SSOT パイプライン詳細](#s1-ssot-パイプライン詳細)
- [§S2 sapphillon-cli コマンド仕様](#s2-sapphillon-cli-コマンド仕様)
- [§S3 静的解析仕様](#s3-静的解析仕様)
- [§S4 バンドルアーキテクチャ詳細](#s4-バンドルアーキテクチャ詳細)
- [§S5 パッケージ検証フロー詳細](#s5-パッケージ検証フロー詳細)
- [§S6 Sapphillon 固有データスキーマ (repository.proto)](#s6-sapphillon-固有データスキーマ-repositoryproto)
- [§S7 TUF メタデータマッピング例](#s7-tuf-メタデータマッピング例)
- [§S8 URL ポリシー](#s8-url-ポリシー)
- [§S9 arcana-cli コマンド仕様](#s9-arcana-cli-コマンド仕様)
- [§S10 PR 検証項目](#s10-pr-検証項目)
- [§S11 公式 API エンドポイント仕様](#s11-公式-api-エンドポイント仕様)
- [§S12 root 鍵ローテーション詳細フロー](#s12-root-鍵ローテーション詳細フロー)
- [§S13 所有権移転詳細フロー](#s13-所有権移転詳細フロー)
- [§S14 オンボーディング PR CI 検証](#s14-オンボーディング-pr-ci-検証)
- [§S15 timestamp cron アラート詳細](#s15-timestamp-cron-アラート詳細)
- [§S16 開発者鍵の運用](#s16-開発者鍵の運用)

---

## §S1. SSOT パイプライン詳細

> architecture.md §3 を参照。

### パイプラインステップ

1. **源流:** `repository.proto` に Sapphillon 固有の全データ構造を定義する（完全版は [§S6](#s6-sapphillon-固有データスキーマ-repositoryproto)）。
2. **Rust 構造体化:** `prost` クレートにより、Proto から Rust の構造体を自動生成する。
3. **外部スキーマ生成 (自動化):**
    - **JSON Schema:** 生成された Rust 構造体から `schemars` クレートを用いて `schema.json` を出力。これを `package.toml` に適用し、VS Code 等での入力補完と検証を実現。
    - **TypeScript 型定義:** 構造体から `.d.ts` を生成し、プラグイン開発者に `@sapphillon/types` 等として提供。
4. **TUF メタデータへの統合:**
    - Sapphillon 固有データは JSON に変換され、`Target.custom["sapphillon"]` に格納される。
    - `package.toml` の静的情報と JSDoc から抽出された動的情報（permissions 等）をマージし、完全な `PluginManifest` を構築して `package.js` に埋め込む。

### Proto3 制約の補完

proto3 だけでは表現できない制約（SemVer 形式、正規表現、HTTPS 制約等）は、domain model の validation trait で検証する:

```rust
trait ArcanaValidate {
    fn validate_semantics(&self) -> Result<(), ValidationError>;
}
```

検証項目:
- SemVer 形式（`version`）
- package_id 正規表現
- GitHub ID の形式
- URL が HTTPS であること
- tag の最大数・最大長
- permission kind の許可値
- permission target の構文
- `read` / `write` は絶対パスまたは `*`
- `net` は `<hostname>[:<port>]` / IP / `*`
- `run` はコマンド名または `*`
- `env` は変数名または `*`
- `sys` は API kind (`hostname`, `systemMemoryInfo` 等) または `*`
- `ffi` はパスまたは `*`
- enum の unknown 値拒否
- repeated field の重複禁止
- 関数名の重複禁止

---

## §S2. sapphillon-cli コマンド仕様

> architecture.md §4.1 を参照。

| コマンド | 詳細 |
|---|---|
| `init` | `package.toml` とエントリポイントのテンプレート生成 |
| `keygen` | プラグイン開発者用の **TUF ed25519 鍵ペア** を生成する。この鍵は delegated targets への署名に使用される。 |
| `check` | ソースコードとメタデータの静的解析を実行し、エラーをフィードバックする。 |
| `build` | `check` をパスしたコードのみをバンドルし、`package.js` を出力する。さらに、該当バージョンの delegated targets エントリ（`TargetCustom` + ハッシュ）を生成・更新し、開発者鍵で署名する。 |

---

## §S3. 静的解析仕様

> architecture.md §4.2 を参照。

### フォーマット検証

- `package.toml` の必須フィールド整合性（JSON Schema 相当の検証）。

### JSDoc 構文検証

- `@param`, `@returns`, `@permission` の記述漏れや構文エラーを AST レベルで検出。

### 権限 (Permission) 検証

`@permission` で宣言された権限が、以下のホワイトリストと構文ルールに合致するか厳密に検証する。権限の kind は Deno の Permission モデルに準拠する:

| kind | Deno flag | target 形式 |
|---|---|---|
| `read` | `--allow-read` | 絶対パス or `*` |
| `write` | `--allow-write` | 絶対パス or `*` |
| `net` | `--allow-net` | `<hostname>[:<port>]`, IP or `*` |
| `run` | `--allow-run` | コマンド名 or `*` |
| `env` | `--allow-env` | 変数名 or `*` |
| `sys` | `--allow-sys` | API kind (`hostname`, `systemMemoryInfo` 等) or `*` |
| `ffi` | `--allow-ffi` | パス (dynamic library) or `*` (unstable) |

> `import` Permission は静的バンドル前提により不要 (runtime でリモート import は発生しない)。

### identifier 検証ルール

| 対象 | 正規表現 | 備考 |
|---|---|---|
| `author_slug` | `^[a-z0-9](?:[a-z0-9-]{0,38})$` | GitHub username 互換 |
| `package_id` | `^[a-z0-9](?:[a-z0-9-]{0,63})$` | |
| `version` | SemVer 2.0.0 strict | build metadata の path 許可は要検討 |
| `tag` | `^[a-z0-9][a-z0-9-]{0,31}$` | max 10 tags |
| `function_name` | `^[a-zA-Z_$][a-zA-Z0-9_$]*$` | JS 識別子 |

**共通拒否リスト:**

- path traversal: `../`, `..\`, `%2e%2e`, `%2E%2E`
- backslash, control characters, null byte
- Unicode confusables (Confusable detection を通す)
- 空文字列
- 先頭/末尾の dot, hyphen

### 禁止構文ポリシー

以下の構文は静的解析 (`sapphillon-cli check`) で検出・拒否する:

- `eval` / `Function` constructor
- dynamic import (静的文字列のみ許可、それ以外は拒否)

> external import (`http(s):`, `npm:`, `jsr:`) の扱い、`WebAssembly` / `Worker` 生成の扱いは §S4 の import ポリシーおよび Phase 2 で決定する。本セクションでは重複して記述しない。

> **静的バンドルによる保証:** `package.js` はビルド時に完全にバンドルされた単一ファイルである。dynamic import / remote import はバンドラ (`deno_emit`) がビルド時にエラーを出すことで自然に強制される。`eval` / `Function` constructor は AST 解析 (`sapphillon-cli check`) で検出・拒否する。runtime における dynamic import / remote import のリスクは原理的に排除されており、runtime sandbox は capability enforcement (OS/FS/Net への直接アクセス遮断) に集中できる。

---

## §S4. バンドルアーキテクチャ詳細

> architecture.md §4.3 を参照。

### バンドル手順

1. **専用エントリラッパーの動的生成:** ビルド時、ユーザーが指定したエントリポイントに対して、公開 API と抽出したメタデータ(`PluginManifest`)のみを安全に `globalThis.__SAPPHILLON_ARCANA_META__.package` に束ねるラッパーコードをメモリ上に自動生成する。
2. **バンドル処理:** `deno_graph` と `deno_emit` を用いて、仮想ラッパーコードを起点としてバンドルを実行する。
3. **マニフェスト直列化:** `PluginManifest` をコメントブロックとして `package.js` に埋め込む（確定的な抽出を可能にするため）。詳細は後述の manifest 仕様。
4. **AST 整合性テストの徹底:** 変換後コードの安定性を担保するため、エッジケースに対する AST レベルの結合テストを `sapphillon-cli` 側に用意する。
5. **delegated targets エントリ生成:** 完成した `package.js` のハッシュ（SHA-256）・長さを計算し、`TargetCustom`（[§S6](#s6-sapphillon-固有データスキーマ-repositoryproto)）とともに開発者の delegated targets（`<author_github_id>.json`）に新エントリを追加して署名する。

### manifest ブロック抽出仕様

```
形式:
  /*! SAPPHILLON_ARCANA_MANIFEST_V1 <base64url(canonical-json)> */
  (JSON 直接埋め込みではなく base64url 化でパーサを単純化)

配置:
  package.js の先頭 64KiB 以内に1つだけ存在

ルール:
  - 複数存在した場合: reject
  - 最大サイズ: manifest JSON 256KiB
  - JSON: canonical JSON (key 昇順、UTF-8)
  - 抽出後: schema validation + semantic validation を実行
  - 文字列リテラル内の sentinel 誤検出防止: コメント構文で厳密マッチ
```

### deno_ast / deno_graph / deno_emit バージョン追従方針

Sapphillon Arcana v1.0 の Deno クレートバージョン:

| crate | 採用バージョン | 根拠 |
|---|---|---|
| `deno_ast` | `0.53.2` | denoland/deno 本体と同一 (transpiling feature 有効) |
| `deno_graph` | `0.108.1` | denoland/deno 本体に追従 (default-features = false) |
| `deno_emit` | `0.46.0` | 最新版。Deno 本体では直接依存していないがバンドル機能に必要 |

追従ルール:
- Deno 本体 (denoland/deno) の Cargo.toml `[workspace.dependencies]` を確認し、deno_ast / deno_graph は Deno 本体のバージョンに合わせる
- deno_emit は最新安定版を使用する (Deno 本体の直接依存ではないため)
- `Cargo.lock` でバージョンを固定（再現可能ビルド）
- minor update 時は AST 結合テストを全実行

> `deno_emit` の最終更新が 2024-10-29 と古い。バンドル機能の代替として `deno_graph` 単体で emit 可能かを Phase 2 で評価する。

plugin build での import ポリシー:

- external import (`http(s):`, `npm:`, `jsr:`) はビルド時に全てバンドルされ `package.js` に埋め込まれる。配布後の外部依存解決は発生しない。個別の方針は Phase 2 で決定する
- TypeScript target: ES2022
- module resolution: bundler resolution

---

## §S5. パッケージ検証フロー詳細

> architecture.md §5.2 を参照。

### 検証ステップ詳細

```text
1. root.json        検証（自己署名 + expires）          → 全ロール鍵の信頼起点
2. timestamp.json   検証（timestamp 鍵）+ expires       → リプレイ対策
3. snapshot.json    検証（snapshot 鍵）                  → 全メタデータ版数一貫性
4. targets.json     検証（targets 鍵）                   → 委譲先発見
5. <author>.json    検証（開発者鍵 = delegated targets） → 開発者署名
6. package.js 取得:
   6a. TUF target path に対応する TargetDescription を tough で検証
   6b. Sapphillon 独自 TargetFetcher が custom.download_url から取得
   6c. 取得した bytes の length / sha256 を TargetDescription と照合
7. PluginManifest   抽出（/* @arcana-manifest {...} */） → package.js 内
8. 整合性チェック    manifest.functions[*].permissions
                   == custom.function_permissions        → 権限昇格攻撃防御
9. RepoOverride     repo-policy/overrides.json を TUF target として検証
                   (targets 鍵で署名)                      → 管理者署名のライフサイクル検証
10. ライフサイクル   RepoOverride を適用 (yanked / deprecated /
    判定            abandoned / advisory)                   → install policy 判定
11. install policy  yanked → block / deprecated → warn /
    判定            abandoned → warn / それ以外 → allow     → 最終インストール可否
```

- ステップ 1-5 は `tough` の標準機能で完結する。
- ステップ 6a は TUF target metadata に基づく検証、6b-6c は Sapphillon 独自 transport。
- ステップ 7-8 が Sapphillon 固有の追加検証である。
- ステップ 9-11 は RepoOverride の検証・適用・install policy 判定（全て Sapphillon 固有）。

### TargetFetcher インターフェース案

```rust
trait TargetFetcher {
    async fn fetch_target(
        &self,
        target_path: &str,
        custom: &TargetCustom,
    ) -> Result<Bytes>;
}
```

### PermissionSet 正規化ルール

ステップ8 の等価比較は、正規化済み permission set に対して行う:

```text
- functions は function_name の昇順にソート
- function_name の重複は禁止 (reject)
- permissions は (kind, normalized_target) の昇順にソート
- permission の重複は排除
- Net host: lower-case + IDNA/punycode 正規化、末尾ドット除去、port 正規化
- read/write path: absolute + lexical normalize (.. 解決)
- env: OS ごとの case sensitivity 明示 (Linux: case-sensitive)
- ffi path: absolute + lexical normalize
- sys kind: 正規化された API 名 (lowercase)
- IPv6: 正規化表記 (RFC 5952)
- "*" は wildcard として妥当性のみ検証 (展開しない)
```

---

## §S6. Sapphillon 固有データスキーマ (repository.proto)

> architecture.md §3, §6.4 を参照。
>
> **重要 — 署名責任境界:** `yanked`, `deprecated`, `yank_reason` 等のライフサイクル情報は、リポジトリ管理者が付与するものである。これらを開発者署名の `TargetCustom` 内に置くと、管理者は開発者鍵なしで変更できず運用が破綻する。そのため `TargetCustom` からこれらを分離し、管理者署名領域として `RepoOverrideIndex` / `PackageOverride` を導入する。

```proto
syntax = "proto3";
package arcana.v1;

// ============================================================
// TargetCustom — Target.custom["sapphillon"] に格納
//     （バージョン単位のメタデータ・開発者署名対象）
// ============================================================
message TargetCustom {
  string package_id = 1;
  string package_name = 2;
  string version = 3;                       // SemVer
  string description = 4;
  string author_github_id = 5;              // 代表者1名

  // package.js のダウンロード先（外部ホスティング）
  string download_url = 6;

  // 関数ごとの権限（検証ステップ: マニフェスト整合性チェックで使用）
  repeated FunctionPermissionMeta function_permissions = 7;

  // ※ yanked / yank_reason / deprecated は TargetCustom から削除し、
  //    RepoOverrideIndex / PackageOverride に移管（管理者署名領域）

  repeated string tags = 8;
  optional string homepage_url = 9;

  // Sapphillon スキーマバージョン（TUF spec_version とは別）
  string arcana_schema_version = 10;

  // immutable な GitHub numeric user id (所有権検証用)
  optional string github_numeric_id = 11;
}

// ============================================================
// RepoOverrideIndex — 管理者署名のライフサイクル overlay
//     targets.json の直接ターゲットとして "repo-policy/overrides.json" に配置
//     targets 鍵（管理者鍵）で署名。delegated targets ではなく
//     targets.json の targets フィールドに直接登録する。
// ============================================================
message RepoOverrideIndex {
  repeated PackageOverride overrides = 1;
  string arcana_schema_version = 2;
}

message PackageOverride {
  string target_path = 1;     // packages/<author>/<pkg>/<ver>/package.js
  bool yanked = 2;
  string yank_reason = 3;
  bool deprecated = 4;
  string deprecated_message = 5;
  bool abandoned = 6;
  string advisory_url = 7;
  string updated_at = 8;      // RFC 3339
}

// ============================================================
// 権限関連
// ============================================================
message Permission {
  string kind = 1;       // read | write | net | run | env | sys | ffi (Deno準拠)
  string target = 2;     // kind に応じた target (パス / host / コマンド / 変数名 / API kind / "*")
}

message FunctionPermissionMeta {
  string function_name = 1;
  repeated Permission permissions = 2;
}

// ============================================================
// PluginManifest — package.js 内マニフェスト
//     コメントブロック /* @arcana-manifest {...} */ に直列化
// ============================================================
message IODefinition {
  JSValueType type = 1;
  string name = 2;
  string description = 3;
}

enum JSValueType {
  JS_VALUE_TYPE_UNSPECIFIED = 0;
  JS_VALUE_TYPE_OBJECT = 1;
  JS_VALUE_TYPE_ARRAY = 2;
  JS_VALUE_TYPE_STRING = 3;
  JS_VALUE_TYPE_NUMBER = 4;
  JS_VALUE_TYPE_BOOLEAN = 5;
}

message FunctionDefinition {
  string name = 1;
  string description = 2;
  repeated Permission permissions = 3;
  repeated IODefinition params = 4;
  optional IODefinition returns = 5;        // 単数。void 関数は省略
}

message PluginManifest {
  PackageConfig package_config = 1;
  repeated FunctionDefinition functions = 2;
}

// ============================================================
// PackageConfig — PackageToml / マニフェスト共通
// ============================================================
message PackageConfig {
  string id = 1;
  string name = 2;
  string version = 3;
  string description = 4;
  string author_github_id = 5;              // 単数（代表者1名・username）
  repeated string tags = 6;
  optional string homepage_url = 7;
  optional string github_numeric_id = 8;    // immutable numeric user id (所有権検証用)
}

message PackageToml {
  PackageConfig package = 1;
  string api_version = 2;
}
```

### CLI コマンド分離（yank）

ライフサイクル情報の署名責任境界分離に伴い、yank コマンドを分離する:

| コマンド | 署名鍵 | 対象 |
|---|---|---|
| `arcana-cli repo-yank <target_path> --reason "..."` | targets 鍵（管理者） | `repo-policy/overrides.json` を更新 |
| `sapphillon-cli self-yank <package_id> <version>` | 開発者鍵 | delegated targets の `custom` に `self_yanked` を設定 |

---

## §S7. TUF メタデータマッピング例

> architecture.md §6.4 を参照。

### targets.json（リポジトリ管理者が署名）

```jsonc
{
  "_type": "targets",
  "spec_version": "1.0.0",
  "version": 42,
  "expires": "2027-01-01T00:00:00Z",
  "targets": {
    // 管理者署名のライフサイクル overlay（targets 鍵で署名）
    "repo-policy/overrides.json": {
      "length": 2048,
      "hashes": { "sha256": "fedcba..." }
    }
    // ※ パッケージ実体は全て delegated targets へ委譲
  },
  "delegations": {
    "keys": {
      "<alice_keyid>": { "keytype": "ed25519", "scheme": "ed25519", "keyval": { "public": "..." } },
      "<bob_keyid>":   { "keytype": "ed25519", "scheme": "ed25519", "keyval": { "public": "..." } }
    },
    "roles": [
      { "name": "alice", "keyids": ["<alice_keyid>"], "threshold": 1, "terminating": true,
        "paths": ["packages/alice/*"] },
      { "name": "bob", "keyids": ["<bob_keyid>"], "threshold": 1, "terminating": true,
        "paths": ["packages/bob/*"] }
    ]
  }
}
```

> `spec_version` は TUF 1.0.0 準拠として `"1.0.0"` に統一する（`tough` が受け入れる形式を Phase 0 で確認）。

### delegated targets: `<author>.json`（開発者が署名）

例: `alice.json`

```jsonc
{
  "_type": "targets",
  "spec_version": "1.0.0",
  "version": 7,
  "expires": "2027-01-01T00:00:00Z",
  "targets": {
    "packages/alice/cool-plugin/1.2.0/package.js": {
      "length": 18432,
      "hashes": { "sha256": "abcd..." },
      "custom": {
        "sapphillon": {
          "package_id": "cool-plugin",
          "package_name": "Cool Plugin",
          "version": "1.2.0",
          "author_github_id": "alice",
          "download_url": "https://raw.githubusercontent.com/.../package.js",
          "function_permissions": [ { "function_name": "f", "permissions": [ /* ... */ ] } ],
          "arcana_schema_version": "1.0.0",
          "github_numeric_id": "12345678"
        }
      }
    }
  }
}
```

> 開発者公開鍵は targets.json の `delegations.keys` に格納されるため、旧設計の `author_public_key` に相当するフィールドは不要。同様に、リポジトリ公開鍵は root.json の `roles` に分散格納される。

### repo-policy/overrides.json（管理者署名のライフサイクル overlay）

```jsonc
{
  "overrides": [
    {
      "target_path": "packages/alice/cool-plugin/1.2.0/package.js",
      "yanked": true,
      "yank_reason": "malware report confirmed",
      "deprecated": false,
      "deprecated_message": "",
      "abandoned": false,
      "advisory_url": "https://github.com/.../security/advisories/...",
      "updated_at": "2026-06-25T00:00:00Z"
    }
  ],
  "arcana_schema_version": "1.0.0"
}
```

### snapshot.json の meta 構造（delegated metadata hash 含む）

```jsonc
{
  "meta": {
    "targets.json": {
      "version": 42,
      "length": 1234,
      "hashes": { "sha256": "..." }
    },
    "alice.json": {
      "version": 7,
      "length": 5678,
      "hashes": { "sha256": "..." }
    }
  }
}
```

---

## §S8. URL ポリシー

> architecture.md §6.5 を参照。

### download_url 制限

```
scheme:
  https:// のみ許可
  http, file, data, ftp, blob, ssh, git は拒否

host:
  localhost / 127.0.0.0/8 / ::1 拒否
  private IP (RFC 1918: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16) 拒否
  link-local (169.254.0.0/16) 拒否
  cloud metadata IP (169.254.169.254) 拒否

DNS:
  解決後 IP を検査
  DNS rebinding 対策: 解決前後で IP 一致確認

redirect:
  最大 5 回
  redirect 後 URL も同一 policy で再検査

サイズ・タイムアウト:
  Content-Length 上限: 50MB (設定可能)
  実ダウンロードサイズ上限: 50MB (stream で監視)
  接続タイムアウト: 10s
  全体タイムアウト: 120s
  retry: 最大 3 回 / exponential backoff

その他:
  MIME type: 参考情報扱い、信頼しない (最終的には hash/length で検証)
  User-Agent: 固定値 ("arcana-cli/1.0" 等)
  credential forwarding: 禁止
```

### development / production ポリシー分離

URL ポリシーは適用コンテキストに応じて 2 段階に分ける:

| コンテキスト | ポリシー | 備考 |
|---|---|---|
| **production** (CI PR 検証, クライアント, api-server) | 厳格 (上記の全制限を適用) | SSRF 対策必須 |
| **development** (`sapphillon-cli build --dev` のローカル実行) | 緩和 (`http://`, `localhost`, private IP を許可) | 開発者の自己責任 |

development モード:
- `sapphillon-cli build --dev` で URL ポリシー検証をスキップ
- hash/length 検証は必須 (同一 package.js であることは保証)
- ビルド成果物は開発用であり、manifest に dev マーカーを付与
- dev マーカー付き package.js は CI 検証で reject される (リポジトリ登録不可)

### 適用箇所

- CI PR 検証（[§S10](#s10-pr-検証項目)）
- クライアント（[§S5](#s5-パッケージ検証フロー詳細) ステップ 6b）
- api-server download リダイレクト（[§S11](#s11-公式-api-エンドポイント仕様)）

---

## §S9. arcana-cli コマンド仕様

> architecture.md §6.7 を参照。

| コマンド | 詳細 |
|---|---|
| `init <name>` | リポジトリ管理者用の TUF 鍵ペア群（root/timestamp/snapshot/targets）とディレクトリ構造を生成。 |
| `add-version <url>` | URL からファイルを DL し検証。同一バージョンの別ハッシュが存在する場合は Reject。 |
| `repo-yank <target_path> --reason "..."` | 対象バージョンの yanked を `repo-policy/overrides.json` に設定し、targets 鍵で署名（[§S6](#s6-sapphillon-固有データスキーマ-repositoryproto) の CLI 分離参照）。 |
| `sign-timestamp` | timestamp.json を緊急再署名（cron 失敗時の避難コマンド）。 |
| `build` | TUF メタデータ全体の整合性を検証し、dist/ に出力。 |

---

## §S10. PR 検証項目

> architecture.md §7 を参照。

GitHub Actions により以下の検証が自動で行われる:

| # | 検証項目 | 内容 |
|---|---|---|
| 1 | **開発者署名検証** | PR に含まれる delegated targets が開発者鍵で適切に署名されているか |
| 2 | **ハッシュ照合** | 新規 target の `download_url` から `package.js` を DL し、`hashes` と一致するか（[§S8](#s8-url-ポリシー) の URL ポリシー適用） |
| 3 | **メタデータ整合性** | `package.js` 内 `PluginManifest` の `function_permissions` が `custom.function_permissions` と一致するか（[§S5](#s5-パッケージ検証フロー詳細) の正規化ルール適用） |
| 4 | **パス規約** | 新規 target のパスが `packages/<author>/<pkg>/<ver>/package.js` に従い、PR 作成者の GitHub username と `<author>` が一致するか |
| 5 | **重複チェック** | 同一パス・同一 keyid が既存と衝突しないか |

> PR 検証の `download_url` からの DL には [§S8](#s8-url-ポリシー) の SSRF 対策を適用する。

---

## §S11. 公式 API エンドポイント仕様

> architecture.md §8 を参照。

### エンドポイント分離原則

TUF 署名対象 metadata と未署名の動的 metadata は API レベルで分離する:

```
/metadata/{role}.json    — TUF 署名対象 metadata (標準 TUF 構造)
  root.json, timestamp.json, snapshot.json, targets.json, <author>.json

/v1/packages             — 未署名動的 metadata (参考情報)
  stars, downloads, trends, search

/v1/download             — トラッキング後リダイレクト
```

### クライアント実装ガイドライン

- `/metadata/*` のみを TUF 検証対象とする
- `/v1/*` の情報は install policy に使用しない（表示のみ）

---

## §S12. root 鍵ローテーション詳細フロー

> architecture.md §9.1 を参照。

### 緊急ローテーション（漏洩疑い時）

1. 漏洩疑いの公告（GitHub Security Advisory 発行）
2. 新 root 鍵ペア生成（threshold 構成を維持）
3. 新 root.json を **有効な旧 root 鍵のしきい値** で署名（漏洩していない鍵のみで対処可能なら）
4. 完全漏洩時（threshold 以上が漏洩）: **コールドスタート問題**。TUF 仕様上も未解決のため、out-of-band 公告チャネル（Security Advisory + README + SNS）でクライアントに手動更新を呼びかける。`inject_official_repo()` のバイナリ同梱公式鍵はこのフェイルセーフとして機能

### 公告チャネル（in-band + out-of-band）

| チャネル | 用途 | 形式 |
|---|---|---|
| **in-band** | 新 root.json の `_extra` フィールド | `rotation_notice: { "reason": "...", "date": "...", "advisory_url": "..." }`（署名対象に含まれるため改ざん不可） |
| **out-of-band 1** | GitHub Security Advisory | CVE 発行・深刻度評価 |
| **out-of-band 2** | リポジトリ README + Releases | 人間可読な公告 |
| **out-of-band 3** | （公式のみ）SNS/ブログ | リーチ拡大 |

> **`_extra` round-trip 検証（Phase 0 spike test）:** `tough` で `_extra` フィールド（unknown field）が round-trip できるか未検証。Phase 0 で確認する。
>
> **`_extra` が使えない場合の代替案:** root.json の `_extra` で配信できない rotation_notice は、root.json 自体の `version` 上昇による検知 + out-of-band 公告チャネル（GitHub Security Advisory / README / SNS）への依存に切り替える。targets ロールの target で root ローテーション公告を代替することはできない（署名する鍵が異なるため）。

---

## §S13. 所有権移転詳細フロー

> architecture.md §9.2 を参照。

### 移転フロー

```text
1. 新所有者が PR 作成
   - targets.json の該当 delegation.role の keyids を自分の鍵 keyid に更新
   - delegations.keys に自分の公開鍵を追加
2. 正当性検証（「任意移転」または「アカウント削除」のみ）
3. 新所有者が該当 delegated targets.json を再署名
   - 既存 target の length/hashes/custom は不変（package.js は再ビルドしない）
   - 署名のみ新鍵で差し替え
4. CI 検証:
   - 全 target の length/hashes が旧版と完全一致（パッケージ実体の不改ざん）
   - 新鍵による delegated targets 署名が有効
   - 新 delegation の path pattern が packages/<author>/* を維持
5. マージ → snapshot.json + timestamp.json 再署名
```

### パッケージ名の占有防止

所有権移転では **同一パス**（`packages/<author>/<pkg>/...`）を維持 → パッケージ名の継続性担保。新規パッケージ名の予約/取り下げは別課題。必要なら `deprecated` + 新パスへ誘導する方式。

### 放棄パッケージの警告（移転なし）

| 項目 | 方針 |
|---|---|
| 放棄判定 | 以下のすべてを満たす場合:
  1. 最後の有意な活動（リリース、Issue/PR への応答、コミット）から6ヶ月以上経過
  2. リポジトリ管理者が旧所有者に2週間以上の応答猶予を与え、応答がない
  3. アクティブなメンテナンスの意思が確認できない
  ※ パッケージが安定しており更新が不要なだけの場合は、旧所有者が応答すれば放棄判定を回避できる |
| 警告の格納先 | `repo-policy/overrides.json` の `abandoned` フラグ（リポジトリ管理者が署名） |
| クライアント挙動 | 該当パッケージのインストール/更新時に **警告を表示**。インストール自体はブロックしない（ユーザー判断） |
| 自動更新 | 警告対象パッケージの自動更新は停止（ユーザーへの通知のみ） |
| 解除 | 旧所有者が活動を再開し新バージョンをリリースした場合、`abandoned` フラグを解除 |

> この運用により、放棄パッケージの **悪意的な引き取り（hijacking）** を完全に防止しつつ、ユーザーへリスクを透明に伝える。

---

## §S14. オンボーディング PR CI 検証

> architecture.md §9.3 を参照。

### PR 内容

`targets.json` への追加:

```jsonc
"delegations": {
  "keys": { "<new_keyid>": { "keytype": "ed25519", ... } },
  "roles": [
    // ... 既存 ...
    { "name": "<github_username>", "keyids": ["<new_keyid>"],
      "threshold": 1, "terminating": true,
      "paths": ["packages/<github_username>/*"] }
  ]
}
```

### CI 検証項目（必須）

| # | 検証項目 | 内容 |
|---|---|---|
| 1 | 鍵フォーマット | ed25519、公開鍵長・エンコーディング正当性 |
| 2 | path 一致性 | `paths` の `<github_id>` が **PR 作成者の GitHub username** と完全一致 |
| 3 | 重複チェック | 同一 github_id / keyid が既存 delegation と衝突しない |
| 4 | Identity Verified | PR 作成者の GitHub アイデンティティが検証済み（実装方式は後述の「Identity Verification 実装方式」に準拠） |
| 5 | **鍵所有証明** | 新鍵で CI が提示したチャレンジ文字列に署名させ、公開鍵で検証。鍵を本当に所持していることの証明 |
| 6 | targets.json 署名 | PR の targets.json が既存 targets 鍵で再署名されていること（管理者が最終的に実施） |

### Identity Verification 実装方式

公式リポジトリでは GitHub App を使用する:

```text
1. GitHub App が PR author の GitHub user id を取得
2. challenge string を PR comment に bot が投稿
3. 開発者が sapphillon-cli sign-challenge で署名
4. 署名結果を PR comment として提出
5. CI/bot が公開鍵で検証
```

fork PR の場合: secrets 制限があるため、label-based workflow または maintainers による手動チャレンジ検証に fallback。

### 承認ポリシー

- 既存リポジトリメンテナ **1名以上** の approval 必須
- 自動マージは不可（人間による審査）

---

## §S15. timestamp cron アラート詳細

> architecture.md §9.4 を参照。

### 多層検知

| 層 | 仕組み | 検知対象 |
|---|---|---|
| **L1: Actions 内部** | workflow `if: failure()` で通知ステップ起動 → GitHub Issue 自動作成 or Slack/Email webhook | workflow ジョブ自体の失敗 |
| **L2: 外部監視** | cron とは別系統の HTTP 監視（UptimeRobot / 自前 probe）。`timestamp.json` を GET し `expires` が `現在時刻 + 12時間` 以降であることを確認 | cron が silent fail、quota 枯渇、GitHub Actions 障害 |
| **L3: クライアント側** | インストール/更新時に timestamp `expires` 切れを検知 → ユーザーへ「リポジトリが古い可能性」を明示。**新規インストールはブロック**、既存は動作継続、自動更新は停止 | 全環境のフェイルセーフ |

### 監視対象（timestamp.json 以外も含む）

```text
- timestamp.json expires (12時間切ったら L2 アラート)
- snapshot.json expires
- targets.json expires
- 各 delegated targets expires
- root.json expires
```

> 実運用では「delegated metadata が期限切れで作者全体が使えない」事故の方が起きやすいため、timestamp.json 以外の監視も必須。

### 推奨監視しきい値

- GitHub Actions の再試行: 3回失敗で L1 アラート
- L2 probe 間隔: 15分

### 復旧フロー

1. アラート検知 → 管理者が cron 設定/quota/鍵を確認
2. timestamp 鍵で timestamp.json を手動再署名（`arcana-cli sign-timestamp` 避難コマンド）
3. snapshot.json も必要に応じて再署名
4. commit & push で静的ホストへ反映

---

## §S16. 開発者鍵の運用

> architecture.md §9.5 を参照。

### 紛失時

1. targets.json の delegations から当該 keyid を削除（管理者 PR）
2. 新鍵を生成し、delegated targets を新鍵で再署名
3. 旧鍵で署名された既存パッケージは hash 不変で再署名のみ差し替え

### 漏洩時

1. 即座に targets.json から当該 delegation を削除（緊急 PR）
2. 悪意ある更新を防ぐため、snapshot/timestamp を即時再署名
3. 必要に応じて RepoOverride で当該パッケージを yank

### 複数鍵 / threshold

- delegated role の threshold はデフォルト 1
- 重要パッケージは threshold > 1 を許可（複数鍵登録可能）
- organization package は複数署名者を delegated role に登録可能
