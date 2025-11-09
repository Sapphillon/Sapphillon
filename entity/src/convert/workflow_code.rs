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

use crate::entity::workflow_code::Model as EntityWorkflowCode;
use sapphillon_core::proto::sapphillon::v1::WorkflowCode as ProtoWorkflowCode;
use sapphillon_core::proto::sapphillon::v1::{
    WorkflowResult as ProtoWorkflowResult,
    PluginPackage as ProtoPluginPackage,
};

/// Convert an entity `workflow_code::Model` into the corresponding
/// proto `WorkflowCode` message.
///
/// This is intentionally a plain function (not an Into/From impl) so
/// callers can invoke an explicit conversion without relying on trait
/// resolution or implicit conversions.
pub fn workflow_code_to_proto(entity: &EntityWorkflowCode) -> ProtoWorkflowCode {
    // Map optional created_at (chrono::DateTime<Utc>) to protobuf Timestamp
    let created_at = entity.created_at.map(|dt| {
        sapphillon_core::proto::google::protobuf::Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    });

    ProtoWorkflowCode {
        id: entity.id.clone(),
        // workflow_id is stored in the DB entity but the proto message for
        // WorkflowCode intentionally omits it; keep fields consistent with
        // other conversions in the codebase.
        code_revision: entity.code_revision,
        code: entity.code.clone(),
        language: entity.language,
        created_at,
        // The following fields are left empty by default; callers may
        // populate them separately when they have joined/loaded relations.
        result: Vec::new(),
        plugin_packages: Vec::new(),
        plugin_function_ids: Vec::new(),
        allowed_permissions: Vec::new(),
    }
}

/// Convert an entity `workflow_code::Model` into the corresponding proto
/// `WorkflowCode` message, optionally attaching relation vectors when the
/// caller has already loaded related records.
pub fn workflow_code_to_proto_with_relations(
    entity: &EntityWorkflowCode,
    result: Option<&[ProtoWorkflowResult]>,
    plugin_packages: Option<&[ProtoPluginPackage]>,
    plugin_function_ids: Option<&[String]>,
    allowed_permissions: Option<&[sapphillon_core::proto::sapphillon::v1::AllowedPermission]>,
) -> ProtoWorkflowCode {
    let mut p = workflow_code_to_proto(entity);

    if let Some(r) = result {
        p.result = r.to_vec();
    }

    if let Some(pp) = plugin_packages {
        p.plugin_packages = pp.to_vec();
    }

    if let Some(pf_ids) = plugin_function_ids {
        p.plugin_function_ids = pf_ids.to_vec();
    }

    if let Some(ap) = allowed_permissions {
        p.allowed_permissions = ap.to_vec();
    }

    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::workflow_code::Model as EntityWorkflowCode;
    use sapphillon_core::proto::sapphillon::v1::{
        PluginPackage as ProtoPluginPackage,
        Permission, PermissionLevel, PermissionType,
        AllowedPermission as ProtoAllowedPermission,
    };

    #[test]
    fn converts_minimal_entity_to_proto() {
        let e = EntityWorkflowCode {
            id: "wc1".to_string(),
            workflow_id: "wf1".to_string(),
            code_revision: 1,
            code: "print('hi')".to_string(),
            language: 0,
            created_at: None,
        };

        let p = workflow_code_to_proto(&e);

        assert_eq!(p.id, e.id);
        assert_eq!(p.code_revision, e.code_revision);
        assert_eq!(p.code, e.code);
        assert_eq!(p.language, e.language);
        assert!(p.created_at.is_none());
    }

    #[test]
    fn converts_with_relations_attached() {
        let e = EntityWorkflowCode {
            id: "wc2".to_string(),
            workflow_id: "wf2".to_string(),
            code_revision: 2,
            code: "print('bye')".to_string(),
            language: 1,
            created_at: None,
        };

        let pkg = ProtoPluginPackage {
            package_id: "pkg1".to_string(),
            package_name: "P".to_string(),
            description: "".to_string(),
            functions: Vec::new(),
            package_version: "v1".to_string(),
            deprecated: None,
            plugin_store_url: "BUILTIN".to_string(),
            internal_plugin: Some(true),
            installed_at: None,
            updated_at: None,
            verified: Some(true),
        };

        let allowed = ProtoAllowedPermission {
            plugin_function_id: "pf1".to_string(),
            permissions: vec![Permission {
                display_name: "X".to_string(),
                description: "D".to_string(),
                permission_type: PermissionType::FilesystemRead as i32,
                permission_level: PermissionLevel::Unspecified as i32,
                resource: vec![],
            }],
        };

        let p = workflow_code_to_proto_with_relations(
            &e,
            None,
            Some(&[pkg.clone()]),
            Some(&["pf1".to_string()]),
            Some(&[allowed.clone()]),
        );

        assert_eq!(p.id, e.id);
        assert_eq!(p.plugin_packages.len(), 1);
        assert_eq!(p.plugin_function_ids.len(), 1);
        assert_eq!(p.allowed_permissions.len(), 1);
    }
}