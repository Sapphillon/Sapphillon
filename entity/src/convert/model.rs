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

use crate::entity::model::Model as EntityModel;
use sapphillon_core::proto::sapphillon::ai::v1::Models as ProtoModel;

impl From<EntityModel> for ProtoModel {
    fn from(entity: EntityModel) -> Self {
        ProtoModel {
            name: entity.name,
            display_name: entity.display_name,
            description: entity.description,
            provider_name: entity.provider_name,
        }
    }
}

impl From<ProtoModel> for EntityModel {
    fn from(proto: ProtoModel) -> Self {
        EntityModel {
            name: proto.name,
            display_name: proto.display_name,
            description: proto.description,
            provider_name: proto.provider_name,
        }
    }
}

impl From<&EntityModel> for ProtoModel {
    fn from(entity: &EntityModel) -> Self {
        entity.clone().into()
    }
}

impl From<&ProtoModel> for EntityModel {
    fn from(proto: &ProtoModel) -> Self {
        proto.clone().into()
    }
}

/// Converts a model entity reference into its proto representation.
pub fn model_entity_to_proto(entity: &EntityModel) -> ProtoModel {
    entity.into()
}

/// Converts a model proto reference into the corresponding entity model.
pub fn model_proto_to_entity(proto: &ProtoModel) -> EntityModel {
    proto.into()
}

/// Maps a slice of model entities to proto messages for response payloads.
pub fn model_entities_to_proto(entities: &[EntityModel]) -> Vec<ProtoModel> {
    entities.iter().map(model_entity_to_proto).collect()
}

#[cfg(test)]
mod tests {
    use super::{model_entities_to_proto, model_entity_to_proto, model_proto_to_entity};
    use crate::entity::model::Model as EntityModel;
    use sapphillon_core::proto::sapphillon::ai::v1::Models as ProtoModel;

    #[test]
    fn entity_proto_roundtrip() {
        let entity = EntityModel {
            name: "models/test".to_string(),
            display_name: "Test".to_string(),
            description: Some("desc".to_string()),
            provider_name: "providers/base".to_string(),
        };

        let proto: ProtoModel = entity.clone().into();
        assert_eq!(proto.name, entity.name);
        assert_eq!(proto.display_name, entity.display_name);
        assert_eq!(proto.description, entity.description);
        assert_eq!(proto.provider_name, entity.provider_name);

        let via_helper = model_entity_to_proto(&entity);
        assert_eq!(via_helper, proto);

        let entity_round: EntityModel = proto.clone().into();
        assert_eq!(entity_round, entity);

        let entity_via_helper = model_proto_to_entity(&proto);
        assert_eq!(entity_via_helper, entity);

        let batch = model_entities_to_proto(std::slice::from_ref(&entity));
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0], proto);
    }

    #[test]
    fn proto_entity_roundtrip_empty_description() {
        let proto = ProtoModel {
            name: "".to_string(),
            display_name: "".to_string(),
            description: None,
            provider_name: "providers/any".to_string(),
        };

        let entity: EntityModel = proto.clone().into();
        assert_eq!(entity.name, proto.name);
        assert_eq!(entity.description, proto.description);

        let via_helper = model_proto_to_entity(&proto);
        assert_eq!(via_helper, entity);

        let proto_round = model_entity_to_proto(&entity);
        assert_eq!(proto_round, proto);
    }
}
