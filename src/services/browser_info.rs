// Sapphillon
// BrowserInfoService implementation using BrowserBridge

use std::{sync::Arc, time::Duration};

use floorp_grpc::browser_info as pb;
use pb::browser_info_service_server::{BrowserInfoService, BrowserInfoServiceServer};
use pb::{GetAllContextDataRequest, GetAllContextDataResponse};
use serde_json::json;

use crate::services::browser_bridge::BridgeHub;

pub struct MyBrowserInfoService {
    hub: Arc<BridgeHub>,
    timeout: Duration,
}

impl MyBrowserInfoService {
    pub fn new(timeout: Duration) -> Self {
        Self { hub: BridgeHub::shared().clone(), timeout }
    }
    pub fn into_server(self) -> BrowserInfoServiceServer<Self> { BrowserInfoServiceServer::new(self) }
}

#[tonic::async_trait]
impl BrowserInfoService for MyBrowserInfoService {
    async fn get_all_context_data(
        &self,
        request: tonic::Request<GetAllContextDataRequest>,
    ) -> Result<tonic::Response<GetAllContextDataResponse>, tonic::Status> {
        let req = request.into_inner();
        log::debug!("[BrowserInfo] GetAllContextData request: params={:?}", req.params);

        // Build JSON argument expected by UI side handler
        let args = match req.params {
            Some(p) => json!({
                "params": {
                    "historyLimit": p.history_limit,
                    "downloadLimit": p.download_limit,
                }
            }),
            None => json!({}),
        };

        let args_json = args.to_string();

        let completed = self
            .hub
            .request("browser_info.getAllContextData", Some(args_json), self.timeout)
            .await?;
        log::debug!(
            "[BrowserInfo] Completed id ok: success={} has_json={}",
            completed.success,
            completed.result_json.is_some()
        );
        // Normalize empty/whitespace result to None
        let normalized = completed
            .result_json
            .as_ref()
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) });

        match (completed.success, normalized) {
            (true, Some(json)) => {
                let res = GetAllContextDataResponse {
                    context_data: Some(json),
                    success: true,
                    error_message: None,
                };
                Ok(tonic::Response::new(res))
            }
            _ => {
                let res = GetAllContextDataResponse {
                    context_data: None,
                    success: false,
                    error_message: completed
                        .error_message
                        .or_else(|| Some("UI returned no data".to_string())),
                };
                Ok(tonic::Response::new(res))
            }
        }
    }
}

// Re-export for server wiring
// Public re-export removed (unused)
