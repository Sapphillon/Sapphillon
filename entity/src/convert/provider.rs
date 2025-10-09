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

use crate::entity::provider::Model as EntityProvider;
use sapphillon_core::proto::sapphillon::ai::v1::Provider as ProtoProvider;

impl From<EntityProvider> for ProtoProvider {
    fn from(entity: EntityProvider) -> Self {
        ProtoProvider {
            name: entity.name,
            display_name: entity.display_name,
            api_key: entity.api_key,
            api_endpoint: entity.api_endpoint,
        }
    }
}

impl From<ProtoProvider> for EntityProvider {
    fn from(proto: ProtoProvider) -> Self {
        EntityProvider {
            name: proto.name,
            display_name: proto.display_name,
            api_key: proto.api_key,
            api_endpoint: proto.api_endpoint,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::provider::Model as EntityProvider;
    use sapphillon_core::proto::sapphillon::ai::v1::Provider as ProtoProvider;

    #[test]
    fn entity_to_proto_and_back() {
        let entity = EntityProvider {
            name: "providers/my_provider".to_string(),
            display_name: "My Provider".to_string(),
            api_key: "secret-key".to_string(),
            api_endpoint: "https://api.example.com".to_string(),
        };

        // Entity -> Proto
        let proto: ProtoProvider = entity.clone().into();
        assert_eq!(proto.name, entity.name);
        assert_eq!(proto.display_name, entity.display_name);
        assert_eq!(proto.api_key, entity.api_key);
        assert_eq!(proto.api_endpoint, entity.api_endpoint);

        // Proto -> Entity (round-trip)
        let entity_round: EntityProvider = proto.into();
        assert_eq!(entity_round, entity);
    }

    #[test]
    fn proto_to_entity_and_back_empty_fields() {
        let proto = ProtoProvider {
            name: "".to_string(),
            display_name: "".to_string(),
            api_key: "".to_string(),
            api_endpoint: "".to_string(),
        };

        // Proto -> Entity
        let entity: EntityProvider = proto.clone().into();
        assert_eq!(entity.name, proto.name);
        assert_eq!(entity.display_name, proto.display_name);
        assert_eq!(entity.api_key, proto.api_key);
        assert_eq!(entity.api_endpoint, proto.api_endpoint);

        // Entity -> Proto (round-trip)
        let proto_round: ProtoProvider = entity.into();
        assert_eq!(proto_round, proto);
    }
}
