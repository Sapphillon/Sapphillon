// Sapphillon
// SPDX-FileCopyrightText: 2025 Yuta Takahashi
// SPDX-License-Identifier: MPL-2.0 OR GPL-3.0-or-later
// Sapphillon
//
//
//
//

use log::info;
use tonic::{Request, Response, Status};

// Import the generated protobuf types
use sapphillon_core::proto::sapphillon::v1::version_service_server::VersionService;
use sapphillon_core::proto::sapphillon::v1::{GetVersionRequest, GetVersionResponse, Version};

#[derive(Debug, Default)]
pub struct MyVersionService;

#[tonic::async_trait]
impl VersionService for MyVersionService {
    /// Returns the running service version derived from crate metadata.
    ///
    /// # Arguments
    ///
    /// * `request` - The gRPC request payload for version lookup (unused).
    ///
    /// # Returns
    ///
    /// Returns a gRPC response containing the semantic version string.
    async fn get_version(
        &self,
        request: Request<GetVersionRequest>,
    ) -> Result<Response<GetVersionResponse>, Status> {
        info!("Got a version request: {request:?}");

        let response = GetVersionResponse {
            version: Some(Version {
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
        };

        Ok(Response::new(response))
    }
}

mod test {

    #[allow(unused_imports)]
    use super::*;

    /// Ensures the version service echoes the crate's package version.
    ///
    /// # Arguments
    ///
    /// This asynchronous test takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns `()` after asserting the returned version matches the build metadata.
    #[tokio::test]
    async fn test_get_version() {
        let service = MyVersionService;
        let request = Request::new(GetVersionRequest {});
        let response = service.get_version(request).await.unwrap().into_inner();
        assert!(response.version.is_some());
        let version = response.version.unwrap();
        // The version string should match the crate version
        assert_eq!(version.version, env!("CARGO_PKG_VERSION"));
    }
}
