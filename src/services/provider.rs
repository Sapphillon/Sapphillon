// Sapphillon
// Copyright 2025 Yuta Takahashi
//
// This file is part of Sapphillon
//
// Sapphillon is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::sync::Arc;

use log::{debug, error};
use sea_orm::{DatabaseConnection, DbErr};
use tonic::{Request, Response, Status};

use database::provider as provider_db;
use entity::entity::provider as provider_entity;
use sapphillon_core::proto::google::rpc::{Code as RpcCode, Status as RpcStatus};
use sapphillon_core::proto::sapphillon::ai::v1::provider_service_server::ProviderService;
use sapphillon_core::proto::sapphillon::ai::v1::{
    CreateProviderRequest, CreateProviderResponse, DeleteProviderRequest, DeleteProviderResponse,
    GetProviderRequest, GetProviderResponse, ListProvidersRequest, ListProvidersResponse, Provider,
    UpdateProviderRequest, UpdateProviderResponse,
};

#[derive(Clone, Debug)]
pub struct MyProviderService {
    db: Arc<DatabaseConnection>,
}

impl MyProviderService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }

    fn sanitize_provider(mut provider: Provider) -> Provider {
        provider.api_key.clear();
        provider
    }

    fn ok_status(message: impl Into<String>) -> Option<RpcStatus> {
        Some(RpcStatus {
            code: RpcCode::Ok as i32,
            message: message.into(),
            details: vec![],
        })
    }

    fn map_db_error(err: DbErr) -> Status {
        error!("Database error occurred while handling provider request: {err:?}");
        Status::internal("database operation failed")
    }
}

#[tonic::async_trait]
impl ProviderService for MyProviderService {
    async fn create_provider(
        &self,
        request: Request<CreateProviderRequest>,
    ) -> Result<Response<CreateProviderResponse>, Status> {
        let req = request.into_inner();
        let incoming = req
            .provider
            .ok_or_else(|| Status::invalid_argument("provider field is required"))?;

        if incoming.display_name.trim().is_empty() {
            return Err(Status::invalid_argument(
                "provider.display_name must not be empty",
            ));
        }
        if incoming.api_key.trim().is_empty() {
            return Err(Status::invalid_argument(
                "provider.api_key must not be empty",
            ));
        }
        if incoming.api_endpoint.trim().is_empty() {
            return Err(Status::invalid_argument(
                "provider.api_endpoint must not be empty",
            ));
        }

        let provider_name = if incoming.name.trim().is_empty() {
            format!("providers/{}", uuid::Uuid::new_v4())
        } else {
            incoming.name.clone()
        };

        let model = provider_entity::Model {
            name: provider_name.clone(),
            display_name: incoming.display_name,
            api_key: incoming.api_key,
            api_endpoint: incoming.api_endpoint,
        };

        provider_db::create_provider(&self.db, model.clone())
            .await
            .map_err(Self::map_db_error)?;

        debug!("Created provider record: {provider_name}");

        let provider_proto: Provider = model.into();
        let response = CreateProviderResponse {
            provider: Some(Self::sanitize_provider(provider_proto)),
            status: Self::ok_status("provider created"),
        };

        Ok(Response::new(response))
    }

    async fn get_provider(
        &self,
        request: Request<GetProviderRequest>,
    ) -> Result<Response<GetProviderResponse>, Status> {
        let req = request.into_inner();
        if req.name.trim().is_empty() {
            return Err(Status::invalid_argument("name must not be empty"));
        }

        let provider = provider_db::get_provider(&self.db, &req.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::not_found(format!("provider '{}' not found", req.name)))?;

        let provider_proto: Provider = provider.into();
        let response = GetProviderResponse {
            provider: Some(Self::sanitize_provider(provider_proto)),
            status: Self::ok_status("provider retrieved"),
        };

        Ok(Response::new(response))
    }

    async fn list_providers(
        &self,
        request: Request<ListProvidersRequest>,
    ) -> Result<Response<ListProvidersResponse>, Status> {
        let req = request.into_inner();

        let page_size = if req.page_size <= 0 {
            None
        } else {
            Some(req.page_size as u32)
        };
        let page_token = if req.page_token.trim().is_empty() {
            None
        } else {
            Some(req.page_token)
        };

        let (providers, next_page_token) =
            provider_db::list_providers(&self.db, page_token, page_size)
                .await
                .map_err(Self::map_db_error)?;

        let providers: Vec<Provider> = providers
            .into_iter()
            .map(|model| {
                let proto: Provider = model.into();
                Self::sanitize_provider(proto)
            })
            .collect();

        let response = ListProvidersResponse {
            providers,
            next_page_token,
            status: Self::ok_status("providers listed"),
        };

        Ok(Response::new(response))
    }

    async fn delete_provider(
        &self,
        request: Request<DeleteProviderRequest>,
    ) -> Result<Response<DeleteProviderResponse>, Status> {
        let req = request.into_inner();
        if req.name.trim().is_empty() {
            return Err(Status::invalid_argument("name must not be empty"));
        }

        let existing = provider_db::get_provider(&self.db, &req.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::not_found(format!("provider '{}' not found", req.name)))?;

        provider_db::delete_provider(&self.db, &existing.name)
            .await
            .map_err(Self::map_db_error)?;

        let response = DeleteProviderResponse {
            status: Self::ok_status("provider deleted"),
        };

        Ok(Response::new(response))
    }

    async fn update_provider(
        &self,
        request: Request<UpdateProviderRequest>,
    ) -> Result<Response<UpdateProviderResponse>, Status> {
        let req = request.into_inner();
        let incoming = req
            .provider
            .ok_or_else(|| Status::invalid_argument("provider field is required"))?;

        if incoming.name.trim().is_empty() {
            return Err(Status::invalid_argument("provider.name must not be empty"));
        }

        let mut existing = provider_db::get_provider(&self.db, &incoming.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::not_found(format!("provider '{}' not found", incoming.name)))?;

        let mask_paths = req.update_mask.map(|mask| mask.paths).unwrap_or_default();
        let update_all = mask_paths.is_empty();

        let desired: provider_entity::Model = incoming.into();

        if update_all || mask_paths.iter().any(|path| path == "display_name") {
            existing.display_name = desired.display_name.clone();
        }
        if update_all || mask_paths.iter().any(|path| path == "api_key") {
            existing.api_key = desired.api_key.clone();
        }
        if update_all || mask_paths.iter().any(|path| path == "api_endpoint") {
            existing.api_endpoint = desired.api_endpoint.clone();
        }

        provider_db::update_provider(&self.db, existing.clone())
            .await
            .map_err(Self::map_db_error)?;

        let updated = provider_db::get_provider(&self.db, &existing.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::internal("provider missing after update"))?;

        let provider_proto: Provider = updated.into();
        let response = UpdateProviderResponse {
            provider: Some(Self::sanitize_provider(provider_proto)),
            status: Self::ok_status("provider updated"),
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::MigratorTrait;
    use sapphillon_core::proto::google::protobuf::FieldMask;

    async fn setup_service() -> MyProviderService {
        let conn = sea_orm::Database::connect("sqlite::memory:?cache=shared")
            .await
            .expect("connect sqlite memory db");
        migration::Migrator::up(&conn, None)
            .await
            .expect("apply migrations");

        MyProviderService::new(conn)
    }

    #[tokio::test]
    async fn create_and_get_provider_roundtrip() {
        let service = setup_service().await;

        let create_req = Request::new(CreateProviderRequest {
            provider: Some(Provider {
                name: String::new(),
                display_name: "Test Provider".to_string(),
                api_key: "super-secret".to_string(),
                api_endpoint: "https://example.test".to_string(),
            }),
        });

        let created = service
            .create_provider(create_req)
            .await
            .expect("create provider")
            .into_inner();

        let provider = created.provider.expect("provider in response");
        assert!(provider.name.starts_with("providers/"));
        assert!(provider.api_key.is_empty(), "api_key should be sanitized");

        let fetched = service
            .get_provider(Request::new(GetProviderRequest {
                name: provider.name.clone(),
                status: None,
            }))
            .await
            .expect("get provider")
            .into_inner();

        let fetched_provider = fetched.provider.expect("provider returned");
        assert_eq!(fetched_provider.name, provider.name);
        assert!(fetched_provider.api_key.is_empty());
        assert_eq!(fetched_provider.display_name, "Test Provider");
    }

    #[tokio::test]
    async fn update_and_list_providers() {
        let service = setup_service().await;

        let create_req = Request::new(CreateProviderRequest {
            provider: Some(Provider {
                name: String::new(),
                display_name: "Initial".to_string(),
                api_key: "secret".to_string(),
                api_endpoint: "https://initial.test".to_string(),
            }),
        });

        let create_resp = service
            .create_provider(create_req)
            .await
            .expect("create")
            .into_inner();
        let created = create_resp.provider.expect("provider");

        let update_req = Request::new(UpdateProviderRequest {
            provider: Some(Provider {
                name: created.name.clone(),
                display_name: "Updated".to_string(),
                api_key: "secret".to_string(),
                api_endpoint: "https://updated.test".to_string(),
            }),
            update_mask: Some(FieldMask {
                paths: vec!["display_name".to_string(), "api_endpoint".to_string()],
            }),
        });

        let update_resp = service
            .update_provider(update_req)
            .await
            .expect("update")
            .into_inner();
        let updated = update_resp.provider.expect("provider");
        assert_eq!(updated.display_name, "Updated");
        assert_eq!(updated.api_endpoint, "https://updated.test");

        let list_resp = service
            .list_providers(Request::new(ListProvidersRequest {
                page_size: 10,
                page_token: String::new(),
            }))
            .await
            .expect("list")
            .into_inner();

        assert_eq!(list_resp.providers.len(), 1);
        let listed = &list_resp.providers[0];
        assert_eq!(listed.display_name, "Updated");
        assert!(listed.api_key.is_empty());
        assert!(list_resp.next_page_token.is_empty());

        service
            .delete_provider(Request::new(DeleteProviderRequest {
                name: created.name.clone(),
            }))
            .await
            .expect("delete");

        let err = service
            .get_provider(Request::new(GetProviderRequest {
                name: created.name.clone(),
                status: None,
            }))
            .await
            .expect_err("get should fail after delete");

        assert_eq!(err.code(), tonic::Code::NotFound);
    }
}
