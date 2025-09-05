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
    browser_info_service_client::BrowserInfoServiceClient,
    GetAllContextDataRequest, GetContextDataParams,
};
use std::sync::{OnceLock, Mutex};
use deno_error::JsErrorBox;

// gRPCサーバーのアドレス。将来的には設定ファイルから読み込むのが望ましいです。
const GRPC_SERVER_ADDRESS: &str = "http://[::1]:50051";

static BROWSER_INFO_CLIENT: OnceLock<Mutex<BrowserInfoServiceClient<tonic::transport::Channel>>> = OnceLock::new();

fn get_or_init_client() -> Option<std::sync::MutexGuard<'static, BrowserInfoServiceClient<tonic::transport::Channel>>> {
    let lock = BROWSER_INFO_CLIENT.get_or_init(|| {
        // Endpoint を lazy に Channel 化（tonic 0.11 では connect_lazy Enable）
        let ch = tonic::transport::Endpoint::from_static("http://127.0.0.1:9") // 未使用ポート
            .connect_lazy();
        Mutex::new(BrowserInfoServiceClient::new(ch))
    });
    lock.lock().ok()
}

/// Sapphillonがこのプラグインを認識するためのエントリーポイントです。
/// プラグインパッケージ（機能の集まり）を返します。
pub fn browser_info_plugin_package() -> CorePluginPackage {
    CorePluginPackage::new(
        "app.floorp.browser_info".to_string(), // パッケージの一意なID
        "Floorp Browser Info".to_string(),     // プラグイン名
        // このパッケージに含まれるプラグイン関数（Deno Op）のリスト
        vec![get_all_context_data_plugin()],
    )
}

/// 個々のプラグイン関数（Deno Op）を定義します。
pub fn get_all_context_data_plugin() -> CorePluginFunction {
    CorePluginFunction::new(
        // Deno Opの一意なID
        "app.floorp.browser_info.get_all_context_data".to_string(),
        "GetAllContextData".to_string(), // 関数名
        "Gets all browser context data.".to_string(), // 関数の説明
        // この関数に紐付けるRustのOp関数を指定
    op2_get_all_context_data(),
        // ワークフロー実行時に初期化するJavaScriptコードを読み込む
        Some(include_str!("00_browser_info.js").to_string()),
    )
}

/// Deno(JavaScript)とRustを繋ぐOp関数。
/// gRPC通信を行うため、非同期 `async` で定義します。
/// `#[op2(async)]` をつけると、Deno側では自動的にPromiseとして扱われます。
// JavaScript から受け取る入力用（snake_case <-> camelCase を意識してフィールド名は JS 側に合わせる）
// JS 側: { historyLimit: number, downloadLimit: number }
#[derive(serde::Deserialize)]
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
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = BrowserInfoServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
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
