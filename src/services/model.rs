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

use database::{model as model_db, provider as provider_db};
use entity::entity::model as model_entity;
use sapphillon_core::proto::google::rpc::{Code as RpcCode, Status as RpcStatus};
use sapphillon_core::proto::sapphillon::ai::v1::model_service_server::ModelService;
use sapphillon_core::proto::sapphillon::ai::v1::{
    CreateModelRequest, CreateModelResponse, DeleteModelRequest, DeleteModelResponse,
    GetModelRequest, GetModelResponse, ListModelsRequest, ListModelsResponse, Models,
    UpdateModelRequest, UpdateModelResponse,
};

#[derive(Clone, Debug)]
pub struct MyModelService {
    db: Arc<DatabaseConnection>,
}

impl MyModelService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }

    fn sanitize_model(model: Models) -> Models {
        model
    }

    fn ok_status(message: impl Into<String>) -> Option<RpcStatus> {
        Some(RpcStatus {
            code: RpcCode::Ok as i32,
            message: message.into(),
            details: vec![],
        })
    }

    fn map_db_error(err: DbErr) -> Status {
        error!("Database error occurred while handling model request: {err:?}");
        Status::internal("database operation failed")
    }

    async fn ensure_provider_exists(&self, provider_name: &str) -> Result<(), Status> {
        provider_db::get_provider(&self.db, provider_name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| {
                Status::not_found(format!(
                    "provider '{}' not found for model operation",
                    provider_name
                ))
            })?;
        Ok(())
    }
}

#[tonic::async_trait]
impl ModelService for MyModelService {
    async fn create_model(
        &self,
        request: Request<CreateModelRequest>,
    ) -> Result<Response<CreateModelResponse>, Status> {
        let req = request.into_inner();
        let incoming = req
            .model
            .ok_or_else(|| Status::invalid_argument("model field is required"))?;

        if incoming.display_name.trim().is_empty() {
            return Err(Status::invalid_argument(
                "model.display_name must not be empty",
            ));
        }
        if incoming.provider_name.trim().is_empty() {
            return Err(Status::invalid_argument(
                "model.provider_name must not be empty",
            ));
        }

        self.ensure_provider_exists(incoming.provider_name.trim())
            .await?;

        let model_name = if incoming.name.trim().is_empty() {
            format!("models/{}", uuid::Uuid::new_v4())
        } else {
            incoming.name.clone()
        };

        let model = model_entity::Model {
            name: model_name.clone(),
            display_name: incoming.display_name,
            description: incoming.description,
            provider_name: incoming.provider_name,
        };

        model_db::create_model(&self.db, model.clone())
            .await
            .map_err(Self::map_db_error)?;

        debug!("Created model record: {model_name}");

        let model_proto: Models = model.into();
        let response = CreateModelResponse {
            model: Some(Self::sanitize_model(model_proto)),
            status: Self::ok_status("model created"),
        };

        Ok(Response::new(response))
    }

    async fn get_model(
        &self,
        request: Request<GetModelRequest>,
    ) -> Result<Response<GetModelResponse>, Status> {
        let req = request.into_inner();
        if req.name.trim().is_empty() {
            return Err(Status::invalid_argument("name must not be empty"));
        }

        let model = model_db::get_model(&self.db, &req.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::not_found(format!("model '{}' not found", req.name)))?;

        let model_proto: Models = model.into();
        let response = GetModelResponse {
            model: Some(Self::sanitize_model(model_proto)),
            status: Self::ok_status("model retrieved"),
        };

        Ok(Response::new(response))
    }

    async fn list_models(
        &self,
        request: Request<ListModelsRequest>,
    ) -> Result<Response<ListModelsResponse>, Status> {
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

        let (models, next_page_token) = model_db::list_models(&self.db, page_token, page_size)
            .await
            .map_err(Self::map_db_error)?;

        let models: Vec<Models> = models
            .into_iter()
            .map(|model| Self::sanitize_model(model.into()))
            .collect();

        let response = ListModelsResponse {
            models,
            next_page_token,
            status: Self::ok_status("models listed"),
        };

        Ok(Response::new(response))
    }

    async fn delete_model(
        &self,
        request: Request<DeleteModelRequest>,
    ) -> Result<Response<DeleteModelResponse>, Status> {
        let req = request.into_inner();
        if req.name.trim().is_empty() {
            return Err(Status::invalid_argument("name must not be empty"));
        }

        let existing = model_db::get_model(&self.db, &req.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::not_found(format!("model '{}' not found", req.name)))?;

        model_db::delete_model(&self.db, &existing.name)
            .await
            .map_err(Self::map_db_error)?;

        let response = DeleteModelResponse {
            status: Self::ok_status("model deleted"),
        };

        Ok(Response::new(response))
    }

    async fn update_model(
        &self,
        request: Request<UpdateModelRequest>,
    ) -> Result<Response<UpdateModelResponse>, Status> {
        let req = request.into_inner();
        let incoming = req
            .model
            .ok_or_else(|| Status::invalid_argument("model field is required"))?;

        if incoming.name.trim().is_empty() {
            return Err(Status::invalid_argument("model.name must not be empty"));
        }

        let mut existing = model_db::get_model(&self.db, &incoming.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::not_found(format!("model '{}' not found", incoming.name)))?;

        let mask_paths = req.update_mask.map(|mask| mask.paths).unwrap_or_default();
        let update_all = mask_paths.is_empty();

        let desired: model_entity::Model = incoming.clone().into();

        if update_all || mask_paths.iter().any(|path| path == "display_name") {
            if desired.display_name.trim().is_empty() {
                return Err(Status::invalid_argument(
                    "model.display_name must not be empty",
                ));
            }
            existing.display_name = desired.display_name.clone();
        }

        if update_all || mask_paths.iter().any(|path| path == "description") {
            existing.description = desired.description.clone();
        }

        if update_all || mask_paths.iter().any(|path| path == "provider_name") {
            if desired.provider_name.trim().is_empty() {
                return Err(Status::invalid_argument(
                    "model.provider_name must not be empty",
                ));
            }
            self.ensure_provider_exists(desired.provider_name.trim())
                .await?;
            existing.provider_name = desired.provider_name.clone();
        }

        model_db::update_model(&self.db, existing.clone())
            .await
            .map_err(Self::map_db_error)?;

        let updated = model_db::get_model(&self.db, &existing.name)
            .await
            .map_err(Self::map_db_error)?
            .ok_or_else(|| Status::internal("model missing after update"))?;

        let model_proto: Models = updated.into();
        let response = UpdateModelResponse {
            model: Some(Self::sanitize_model(model_proto)),
            status: Self::ok_status("model updated"),
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::MigratorTrait;
    use sapphillon_core::proto::google::protobuf::FieldMask;

    async fn setup_service_with_providers(
        providers: Vec<entity::entity::provider::Model>,
    ) -> MyModelService {
        let conn = sea_orm::Database::connect("sqlite::memory:?cache=shared")
            .await
            .expect("connect sqlite memory db");
        migration::Migrator::up(&conn, None)
            .await
            .expect("apply migrations");

        for provider in providers {
            database::provider::create_provider(&conn, provider)
                .await
                .expect("seed provider");
        }

        MyModelService::new(conn)
    }

    fn provider(name: &str, display_name: &str) -> entity::entity::provider::Model {
        entity::entity::provider::Model {
            name: name.to_string(),
            display_name: display_name.to_string(),
            api_key: "key".to_string(),
            api_endpoint: "https://example.test".to_string(),
        }
    }

    #[tokio::test]
    async fn create_and_get_model_roundtrip() {
        let service = setup_service_with_providers(vec![provider("providers/test", "Test")]).await;

        let create_req = Request::new(CreateModelRequest {
            model: Some(Models {
                name: String::new(),
                display_name: "Sample Model".to_string(),
                description: Some("desc".to_string()),
                provider_name: "providers/test".to_string(),
            }),
        });

        let created = service
            .create_model(create_req)
            .await
            .expect("create model")
            .into_inner();

        let model = created.model.expect("model in response");
        assert!(model.name.starts_with("models/"));
        assert_eq!(model.display_name, "Sample Model");
        assert_eq!(model.description.as_deref(), Some("desc"));

        let fetched = service
            .get_model(Request::new(GetModelRequest {
                name: model.name.clone(),
            }))
            .await
            .expect("get model")
            .into_inner();

        let fetched_model = fetched.model.expect("model returned");
        assert_eq!(fetched_model.name, model.name);
        assert_eq!(fetched_model.display_name, "Sample Model");
        assert_eq!(fetched_model.description.as_deref(), Some("desc"));
    }

    #[tokio::test]
    async fn update_and_list_models() {
        let service = setup_service_with_providers(vec![provider("providers/base", "Base")]).await;

        let create_req = Request::new(CreateModelRequest {
            model: Some(Models {
                name: String::new(),
                display_name: "Initial".to_string(),
                description: Some("detail".to_string()),
                provider_name: "providers/base".to_string(),
            }),
        });

        let create_resp = service
            .create_model(create_req)
            .await
            .expect("create")
            .into_inner();
        let created = create_resp.model.expect("model");

        let update_req = Request::new(UpdateModelRequest {
            model: Some(Models {
                name: created.name.clone(),
                display_name: "Updated".to_string(),
                description: None,
                provider_name: "providers/base".to_string(),
            }),
            update_mask: Some(FieldMask {
                paths: vec!["display_name".to_string(), "description".to_string()],
            }),
        });

        let updated = service
            .update_model(update_req)
            .await
            .expect("update")
            .into_inner()
            .model
            .expect("model");
        assert_eq!(updated.display_name, "Updated");
        assert!(updated.description.is_none());

        let list_resp = service
            .list_models(Request::new(ListModelsRequest {
                page_size: 10,
                page_token: String::new(),
            }))
            .await
            .expect("list")
            .into_inner();

        assert_eq!(list_resp.models.len(), 1);
        let listed = &list_resp.models[0];
        assert_eq!(listed.display_name, "Updated");
        assert!(list_resp.next_page_token.is_empty());
    }

    #[tokio::test]
    async fn delete_model_removes_record() {
        let service = setup_service_with_providers(vec![provider("providers/main", "Main")]).await;

        let create_req = Request::new(CreateModelRequest {
            model: Some(Models {
                name: String::new(),
                display_name: "ToDelete".to_string(),
                description: None,
                provider_name: "providers/main".to_string(),
            }),
        });

        let created = service
            .create_model(create_req)
            .await
            .expect("create")
            .into_inner()
            .model
            .expect("model");

        service
            .delete_model(Request::new(DeleteModelRequest {
                name: created.name.clone(),
            }))
            .await
            .expect("delete");

        let err = service
            .get_model(Request::new(GetModelRequest { name: created.name }))
            .await
            .expect_err("get should fail after delete");

        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn create_model_requires_existing_provider() {
        let service = setup_service_with_providers(vec![]).await;

        let err = service
            .create_model(Request::new(CreateModelRequest {
                model: Some(Models {
                    name: String::new(),
                    display_name: "Invalid".to_string(),
                    description: None,
                    provider_name: "providers/missing".to_string(),
                }),
            }))
            .await
            .expect_err("creation should fail when provider is missing");

        assert_eq!(err.code(), tonic::Code::NotFound);
    }
}
