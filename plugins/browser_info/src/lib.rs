// Copyright 2025 The Floorp Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use deno_core::op2;
use sapphillon_core::plugin::{CorePluginFunction, CorePluginPackage};

// `grpc` クレートから、自動生成されたgRPCの型をインポートします。
use floorp_grpc::browser_info::{
    browser_info_service_client::BrowserInfoServiceClient as BrowserInfoClient,
    GetAllContextDataRequest, GetContextDataParams,
};
use std::sync::{OnceLock, Mutex};
use deno_error::JsErrorBox;
use std::env;

// gRPCサーバーのアドレス。環境変数 `BROWSER_INFO_SERVER_ADDRESS` または `GRPC_SERVER_ADDRESS` で上書き可能。
// 既定は IPv4 ループバックを使用。
const DEFAULT_GRPC_SERVER_ADDRESS: &str = "http://127.0.0.1:50051";

static BROWSER_INFO_CLIENT: OnceLock<Mutex<BrowserInfoClient<tonic::transport::Channel>>> = OnceLock::new();

fn get_or_init_client() -> Option<std::sync::MutexGuard<'static, BrowserInfoClient<tonic::transport::Channel>>> {
    let lock = BROWSER_INFO_CLIENT.get_or_init(|| {
        // Endpoint を lazy に Channel 化（tonic 0.11 では connect_lazy Enable）
        let ch = tonic::transport::Endpoint::from_static("http://127.0.0.1:9") // 未使用ポート
            .connect_lazy();
        Mutex::new(BrowserInfoClient::new(ch))
    });
    lock.lock().ok()
}

/// Sapphillonがこのプラグインを認識するためのエントリーポイントです。
/// プラグインパッケージ（機能の集まり）を返します。
pub fn browser_info_plugin_package() -> CorePluginPackage {
    CorePluginPackage::new(
        "app.floorp.browser_info".to_string(), // パッケージの一意なID
        "Floorp Browser Info".to_string(),     // プラグイン名
        // 非同期版のみを登録（Deno でのランタイム二重起動を避ける）
        vec![get_all_context_data_plugin_async()],
    )
}

/// 個々のプラグイン関数（Deno Op）を定義します。
pub fn get_all_context_data_plugin_async() -> CorePluginFunction {
    CorePluginFunction::new(
        // Deno Opの一意なID（非同期）
        "app.floorp.browser_info.get_all_context_data".to_string(),
        "getAllContextData".to_string(),
        "Gets all browser context data (async).".to_string(),
        op2_get_all_context_data(),
        Some(include_str!("00_browser_info.js").to_string()),
    )
}

/// Deno(JavaScript)とRustを繋ぐOp関数。
/// gRPC通信を行うため、非同期 `async` で定義します。
/// `#[op2(async)]` をつけると、Deno側では自動的にPromiseとして扱われます。
// JavaScript から受け取る入力用（snake_case <-> camelCase を意識してフィールド名は JS 側に合わせる）
// JS 側: { historyLimit: number, downloadLimit: number }
#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InputParams {
    #[serde(default)]
    pub history_limit: Option<i32>,
    #[serde(default)]
    pub download_limit: Option<i32>,
}

#[op2(async)]
#[string]
pub async fn op2_get_all_context_data(#[serde] params: Option<InputParams>) -> Result<String, JsErrorBox> {
    // 環境変数でダミーモードを有効化: BROWSER_INFO_DUMMY=1
    if env::var("BROWSER_INFO_DUMMY").ok().as_deref() == Some("1") {
        // フロント接続確認用のスタブ JSON
        let dummy = serde_json::json!({
            "history": [
                {"title": "Example Site", "url": "https://example.com", "lastVisited": "2025-09-07T12:00:00Z"},
                {"title": "Rust Lang", "url": "https://www.rust-lang.org/", "lastVisited": "2025-09-07T12:05:00Z"}
            ],
            "downloads": [
                {"fileName": "report.pdf", "sizeBytes": 1048576, "status": "completed"}
            ],
            "tabs": [
                {"id": "tab-1", "title": "Start Page", "url": "about:home", "active": true},
                {"id": "tab-2", "title": "Docs", "url": "https://docs.example.com", "active": false}
            ],
            "meta": {
                "dummy": true,
                "generatedAt": "2025-09-07T12:10:00Z",
                "historyLimit": params.as_ref().and_then(|p| p.history_limit),
                "downloadLimit": params.as_ref().and_then(|p| p.download_limit)
            }
        });
        return Ok(dummy.to_string());
    }
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    let addr = env::var("BROWSER_INFO_SERVER_ADDRESS")
        .ok()
        .or_else(|| env::var("GRPC_SERVER_ADDRESS").ok())
        .unwrap_or_else(|| DEFAULT_GRPC_SERVER_ADDRESS.to_string());
    if let Err(e) = BrowserInfoClient::connect(addr).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let request = GetAllContextDataRequest {
        params: params.map(|p| GetContextDataParams { history_limit: p.history_limit, download_limit: p.download_limit }),
    };
    let response = guard.get_all_context_data(request).await
        .map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
        .into_inner();
    if response.success {
        Ok(response.context_data.unwrap_or_else(|| "{}".to_string()))
    } else {
        Err(JsErrorBox::new("Error", response.error_message.unwrap_or_else(|| "Unknown error from server".to_string())))
    }
}

// 同期版はランタイムの制約（block_on不可）により提供しない
