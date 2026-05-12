use std::collections::{BTreeMap, BTreeSet};

use crate::platform::ResourceKind;
use crate::platform::resource::ResourceFacet;

use super::{EdgeDefinition, EntityDefinition, EntityKind};

/// Base fault verbs supported by all entity kinds.
const BASE_ENTITY_FAULTS: &[&str] = &[
    "runtime.fault.call.error",
    "runtime.fault.call.timeout",
    "runtime.fault.timing.delay",
    "runtime.fault.call.block",
    "runtime.fault.scheduler.starve",
    "runtime.fault.resource.exhaust",
    "runtime.fault.resource.quota",
];

/// Base fault verbs supported by all edge kinds.
const BASE_EDGE_FAULTS: &[&str] = BASE_ENTITY_FAULTS;

/// Transport fault verbs for stream and link kinds.
const TRANSPORT_FAULTS: &[&str] = &[
    "runtime.fault.transport.drop",
    "runtime.fault.transport.duplicate",
    "runtime.fault.transport.reorder",
    "runtime.fault.transport.corrupt",
    "runtime.fault.transport.truncate",
    "runtime.fault.transport.partial",
    "runtime.fault.transport.disconnect",
    "runtime.fault.transport.reset",
    "runtime.fault.transport.partition",
    "runtime.fault.transport.blackhole",
    "runtime.fault.transport.throttle",
    "runtime.fault.transport.limit",
];

/// Lifecycle fault verbs for process and thread kinds.
const LIFECYCLE_FAULTS: &[&str] = &[
    "runtime.fault.process.crash",
    "runtime.fault.process.restart",
    "runtime.fault.process.reboot",
];

/// Clock fault verbs for time-bearing kinds.
const CLOCK_FAULTS: &[&str] = &[
    "runtime.fault.clock.jump",
    "runtime.fault.clock.drift",
    "runtime.fault.clock.freeze",
];

/// Durability fault verbs for persistent-state kinds.
const DURABILITY_FAULTS: &[&str] = &["runtime.fault.durability.violate"];

/// Builtin entity kind descriptor.
struct BuiltinEntityKindDescriptor {
    /// Stable kind identifier.
    kind_id: &'static str,
    /// Semantic behavior facets for this kind.
    facets: &'static [ResourceFacet],
}

/// Builtin edge kind descriptor.
struct BuiltinEdgeKindDescriptor {
    /// Stable kind identifier.
    kind_id: &'static str,
    /// Semantic behavior facets for this kind.
    facets: &'static [ResourceFacet],
}

/// Builtin entity kind catalog.
const BUILTIN_ENTITY_KIND_DESCRIPTORS: &[BuiltinEntityKindDescriptor] = &[
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.instance",
        facets: &[ResourceFacet::Clock],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.worker",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.process.instance",
        facets: &[ResourceFacet::Process],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.io.stream",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.fs.inode",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.fs.dentry",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.fs.mount",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.namespace",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.interface",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.connection",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.net.resolver",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.ipc.channel",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.display.surface",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.memory.region",
        facets: &[ResourceFacet::Memory],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.time.clock",
        facets: &[ResourceFacet::Clock],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.security.policy",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.os.service",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.random.stream",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.resource",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.tty.device",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "host.ffi.handle",
        facets: &[],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.error.channel",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindDescriptor {
        kind_id: "runtime.debug.channel",
        facets: &[ResourceFacet::EventSource],
    },
];

/// Builtin edge kind catalog.
const BUILTIN_EDGE_KIND_DESCRIPTORS: &[BuiltinEdgeKindDescriptor] = &[
    BuiltinEdgeKindDescriptor {
        kind_id: "host.fs.parent_child",
        facets: &[],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.fs.fd_binding",
        facets: &[],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.fs.mount_attachment",
        facets: &[],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.net.network_link",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.net.stream_link",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.net.route",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "runtime.instance.owns.worker",
        facets: &[],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "runtime.worker.owns.resource",
        facets: &[],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.ipc.channel",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEdgeKindDescriptor {
        kind_id: "host.process.pipe",
        facets: &[ResourceFacet::Stream],
    },
];

/// Return builtin entity kind definitions.
pub(super) fn builtin_entity_kinds() -> Vec<EntityDefinition> {
    BUILTIN_ENTITY_KIND_DESCRIPTORS
        .iter()
        .map(|descriptor| {
            EntityDefinition::new(descriptor.kind_id)
                .labels(labels_for_kind(descriptor.kind_id))
                .supports_faults(faults_for_facets(BASE_ENTITY_FAULTS, descriptor.facets))
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
                .supports_faults(resource_faults(*resource_kind))
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
                .supports_faults(faults_for_facets(BASE_EDGE_FAULTS, descriptor.facets))
        })
        .collect()
}

/// Return base fault verbs supported by all entity kinds.
pub(super) fn base_supported_entity_faults() -> BTreeSet<String> {
    collect_faults(BASE_ENTITY_FAULTS, &[])
}

/// Return base fault verbs supported by all edge kinds.
pub(super) fn base_supported_edge_faults() -> BTreeSet<String> {
    collect_faults(BASE_EDGE_FAULTS, &[])
}

/// Return the builtin fault set for one resource kind.
fn resource_faults(resource_kind: ResourceKind) -> BTreeSet<String> {
    faults_for_facets(BASE_ENTITY_FAULTS, resource_kind.facets())
}

/// Return one merged builtin fault set.
fn collect_faults(base_faults: &[&str], extra_faults: &[&str]) -> BTreeSet<String> {
    strings(base_faults).chain(strings(extra_faults)).collect()
}

/// Return one fault set derived from semantic facets.
fn faults_for_facets(base_faults: &[&str], facets: &[ResourceFacet]) -> BTreeSet<String> {
    let mut faults = collect_faults(base_faults, &[]);

    for facet in facets {
        match facet {
            ResourceFacet::Storage => faults.extend(strings(DURABILITY_FAULTS)),
            ResourceFacet::Stream | ResourceFacet::EventSource => {
                faults.extend(strings(TRANSPORT_FAULTS));
            }
            ResourceFacet::Process | ResourceFacet::Thread => {
                faults.extend(strings(LIFECYCLE_FAULTS));
            }
            ResourceFacet::Timer | ResourceFacet::Clock => faults.extend(strings(CLOCK_FAULTS)),
            ResourceFacet::Memory => {}
        }
    }

    faults
}

/// Return one iterator over owned fault strings.
fn strings<'a>(faults: &'a [&'a str]) -> impl Iterator<Item = String> + 'a {
    faults.iter().map(|fault| (*fault).to_string())
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

    use super::{
        BUILTIN_EDGE_KIND_DESCRIPTORS, BUILTIN_ENTITY_KIND_DESCRIPTORS, builtin_edge_kinds,
        builtin_entity_kinds, builtin_resource_entity_kinds, resource_faults,
    };
    use crate::platform::ResourceKind;

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
        let socket_faults = resource_faults(ResourceKind::Socket);
        let file_faults = resource_faults(ResourceKind::File);
        let timer_faults = resource_faults(ResourceKind::Timer);
        let process_faults = resource_faults(ResourceKind::Process);
        let shared_memory_faults = resource_faults(ResourceKind::SharedMemory);
        let poll_faults = resource_faults(ResourceKind::Poll);
        let watch_faults = resource_faults(ResourceKind::Watch);

        assert!(socket_faults.contains("runtime.fault.transport.drop"));
        assert!(file_faults.contains("runtime.fault.durability.violate"));
        assert!(timer_faults.contains("runtime.fault.clock.jump"));
        assert!(process_faults.contains("runtime.fault.process.crash"));
        assert!(!shared_memory_faults.contains("runtime.fault.durability.violate"));
        assert!(!poll_faults.contains("runtime.fault.transport.drop"));
        assert!(watch_faults.contains("runtime.fault.transport.drop"));
    }
}
