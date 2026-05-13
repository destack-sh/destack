use std::collections::BTreeMap;

use crate::host::ResourceKind;
use crate::world::policy::{
    base_edge_faults, base_entity_faults, edge_kind_faults, entity_kind_faults,
    resource_kind_faults,
};

use super::{EdgeDefinition, EntityDefinition, EntityKind};

/// Builtin entity kind descriptor.
struct BuiltinEntityKindDescriptor {
    /// Stable kind identifier.
    kind_id: &'static str,
}

/// Builtin edge kind descriptor.
struct BuiltinEdgeKindDescriptor {
    /// Stable kind identifier.
    kind_id: &'static str,
}

/// Builtin entity kind catalog.
const BUILTIN_ENTITY_KIND_DESCRIPTORS: &[BuiltinEntityKindDescriptor] = &[
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.instance",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.worker",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.process.instance",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.io.stream",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.fs.inode",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.fs.dentry",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.fs.mount",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.namespace",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.interface",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.connection",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.resolver",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.ipc.channel",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.display.surface",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.memory.region",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.time.clock",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.security.policy",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.os.service",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.random.stream",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.resource",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.tty.device",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.error.channel",
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.debug.channel",
    },
];

/// Builtin edge kind catalog.
const BUILTIN_EDGE_KIND_DESCRIPTORS: &[BuiltinEdgeKindDescriptor] = &[
    BuiltinEdgeKindDescriptor {
        kind_id: "host.fs.parent_child",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.fs.fd_binding",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.fs.mount_attachment",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.net.network_link",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.net.stream_link",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.net.route",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "runtime.instance.owns.worker",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "runtime.worker.owns.resource",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.ipc.channel",
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.process.pipe",
    },
];

/// Return builtin entity kind definitions.
pub(super) fn builtin_entity_kinds() -> Vec<EntityDefinition> {
    BUILTIN_ENTITY_KIND_DESCRIPTORS
        .iter()
        .map(|descriptor| {
            EntityDefinition::new(descriptor.kind_id)
                .labels(labels_for_kind(descriptor.kind_id))
                .supports_faults(entity_kind_faults(descriptor.kind_id))
        })
        .collect()
}

/// Return builtin resource entity kind definitions.
pub(super) fn builtin_resource_entity_kinds() -> Vec<EntityDefinition> {
    ResourceKind::all()
        .iter()
        .map(|resource_kind| {
            let kind_id = resource_kind.kind_id();
            EntityDefinition::new(kind_id)
                .labels(labels_for_kind(kind_id))
                .supports_faults(resource_kind_faults(*resource_kind))
        })
        .collect()
}

/// Return builtin edge kind definitions.
pub(super) fn builtin_edge_kinds() -> Vec<EdgeDefinition> {
    BUILTIN_EDGE_KIND_DESCRIPTORS
        .iter()
        .map(|descriptor| {
            EdgeDefinition::new(descriptor.kind_id)
                .labels(labels_for_kind(descriptor.kind_id))
                .supports_faults(edge_kind_faults(descriptor.kind_id))
        })
        .collect()
}

/// Return base fault verbs supported by all entity kinds.
pub(super) fn base_supported_entity_faults() -> std::collections::BTreeSet<String> {
    base_entity_faults()
}

/// Return base fault verbs supported by all edge kinds.
pub(super) fn base_supported_edge_faults() -> std::collections::BTreeSet<String> {
    base_edge_faults()
}

/// Build system labels for one kind id.
fn labels_for_kind(kind_id: &str) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(EntityKind::LABEL_KIND.to_string(), kind_id.to_string());

    labels
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::host::ResourceKind;
    use crate::world::policy::resource_kind_faults;

    use super::{
        BUILTIN_EDGE_KIND_DESCRIPTORS, BUILTIN_ENTITY_KIND_DESCRIPTORS, builtin_edge_kinds,
        builtin_entity_kinds, builtin_resource_entity_kinds,
    };

    /// Ensure builtin entity kind descriptors stay unique.
    #[test]
    fn test_builtin_entity_kind_descriptors_are_unique() {
        // collect all builtin entity kind ids
        let kind_ids = BUILTIN_ENTITY_KIND_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.kind_id)
            .collect::<Vec<_>>();
        let unique_kind_ids = kind_ids.iter().copied().collect::<BTreeSet<_>>();

        // every declared builtin entity kind should be unique
        assert_eq!(kind_ids.len(), unique_kind_ids.len());
        assert_eq!(builtin_entity_kinds().len(), unique_kind_ids.len());
    }

    /// Ensure builtin edge kind descriptors stay unique.
    #[test]
    fn test_builtin_edge_kind_descriptors_are_unique() {
        // collect all builtin edge kind ids
        let kind_ids = BUILTIN_EDGE_KIND_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.kind_id)
            .collect::<Vec<_>>();
        let unique_kind_ids = kind_ids.iter().copied().collect::<BTreeSet<_>>();

        // every declared builtin edge kind should be unique
        assert_eq!(kind_ids.len(), unique_kind_ids.len());
        assert_eq!(builtin_edge_kinds().len(), unique_kind_ids.len());
    }

    /// Ensure resource kinds map one to one onto builtin resource entity kinds.
    #[test]
    fn test_builtin_resource_entity_kinds_cover_all_resource_kinds() {
        // derive the resource kind ids from the canonical resource registry
        let expected_kind_ids = ResourceKind::all()
            .iter()
            .map(|resource_kind| resource_kind.kind_id().to_string())
            .collect::<BTreeSet<_>>();

        // derive the topology kind ids from the builtin resource catalog
        let actual_kind_ids = builtin_resource_entity_kinds()
            .into_iter()
            .map(|kind| kind.kind.0)
            .collect::<BTreeSet<_>>();

        // every resource kind should appear exactly once in builtin topology metadata
        assert_eq!(actual_kind_ids, expected_kind_ids);
    }

    /// Ensure representative resource kinds map to the expected fault families.
    #[test]
    fn test_builtin_resource_fault_profiles_cover_representative_kinds() {
        let socket_faults = resource_kind_faults(ResourceKind::Socket);
        let file_faults = resource_kind_faults(ResourceKind::File);
        let timer_faults = resource_kind_faults(ResourceKind::Timer);
        let process_faults = resource_kind_faults(ResourceKind::Process);
        let shared_memory_faults = resource_kind_faults(ResourceKind::SharedMemory);
        let poll_faults = resource_kind_faults(ResourceKind::Poll);
        let watch_faults = resource_kind_faults(ResourceKind::Watch);

        assert!(socket_faults.contains("runtime.fault.transport.drop"));
        assert!(file_faults.contains("runtime.fault.durability.violate"));
        assert!(timer_faults.contains("runtime.fault.clock.jump"));
        assert!(process_faults.contains("runtime.fault.process.crash"));
        assert!(!shared_memory_faults.contains("runtime.fault.durability.violate"));
        assert!(!poll_faults.contains("runtime.fault.transport.drop"));
        assert!(watch_faults.contains("runtime.fault.transport.drop"));
    }
}
