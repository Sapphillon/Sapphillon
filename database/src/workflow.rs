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

pub mod workflow_code_allowed_permission_crud;
pub mod workflow_code_crud;
pub mod workflow_crud;
pub mod workflow_result_crud;

use entity::entity::plugin_function;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait};
use sapphillon_core::proto::sapphillon::v1::{Workflow, WorkflowCode};


use uuid::Uuid;

pub async fn create_workflow_code(
    db: &DatabaseConnection,
    code: String,
    workflow_id: i32,
    plugin_function_ids: Vec<String>,
    plugin_package_ids: Vec<String>,
) -> Result<WorkflowCode, DbErr> {
    // Build an entity model and delegate insertion to the CRUD helper.
    // Note: workflow IDs are stored as strings in the entity model; convert the
    // incoming integer id to string here. Start code_revision at 1 and set a
    // default language of 0 (WORKFLOW_LANGUAGE_UNSPECIFIED).
    let wc = entity::entity::workflow_code::Model {
        id: Uuid::new_v4().to_string(),
        workflow_id: workflow_id.to_string(),
        code_revision: 1,
        code,
        language: 0,
        created_at: None,
    };
    // Use the internal CRUD helper which returns Result<(), DbErr>.
    workflow_code_crud::create_workflow_code(db, wc.clone()).await?;
    
    // Insert related plugin functions.
    let plugin_function_ids = plugin_function_ids.iter().map(|pf_id| {
        entity::entity::workflow_code_plugin_function::ActiveModel {
            workflow_code_id: Set(wc.id.to_owned()),
            plugin_function_id: Set(pf_id.clone()),
            ..Default::default()
        }
    }).collect::<Vec<_>>();
    entity::entity::workflow_code_plugin_function::Entity::insert_many(plugin_function_ids)
        .on_empty_do_nothing()
        .exec(db)
        .await?;

    // Insert related plugin packages.
    let plugin_package_ids = plugin_package_ids.iter().map(|pp_id| {
        entity::entity::workflow_code_plugin_package::ActiveModel {
            workflow_code_id: Set(wc.id.to_owned()),
            plugin_package_id: Set(pp_id.clone()),
            ..Default::default()
        }
    }).collect::<Vec<_>>();
    entity::entity::workflow_code_plugin_package::Entity::insert_many(plugin_package_ids)
        .on_empty_do_nothing()
        .exec(db)
        .await?;

    // Convert the model into the proto type to return.
    let proto = WorkflowCode {
        id: wc.id,
        code_revision: wc.code_revision,
        code: wc.code,
        language: wc.language,
        created_at: None, // keep None for now; mapping timestamps requires conversion
        result: Vec::new(),
        plugin_packages: Vec::new(),
        plugin_function_ids: Vec::new(),
        allowed_permissions: Vec::new(),
    };

    Ok(proto)
}

pub async fn create_workflow(
    db: &DatabaseConnection,
    workflow_code: String,
) -> Result<Workflow, DbErr> {
    todo!();
}