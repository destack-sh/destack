use std::collections::{BTreeMap, BTreeSet};

use crate::platform::ResourceKind;
use crate::platform::resource::ResourceFacet;
use crate::runtime::world::LABEL_TOPOLOGY_KIND;

use super::{WorldEdgeKindDefinition, WorldEntityKindDefinition};

/// Base fault verbs supported by all entity kinds.
const BASE_ENTITY_FAULTS: &[&str] = &[
    "call.error",
    "call.timeout",
    "timing.delay",
    "call.block",
    "scheduler.starve",
    "resource.exhaust",
    "resource.quota",
];

/// Base fault verbs supported by all edge kinds.
const BASE_EDGE_FAULTS: &[&str] = BASE_ENTITY_FAULTS;

/// Transport fault verbs for stream and link kinds.
const TRANSPORT_FAULTS: &[&str] = &[
    "transport.drop",
    "transport.duplicate",
    "transport.reorder",
    "transport.corrupt",
    "transport.truncate",
    "transport.partial",
    "transport.disconnect",
    "transport.reset",
    "transport.partition",
    "transport.blackhole",
    "transport.throttle",
    "transport.limit",
];

/// Lifecycle fault verbs for process and thread kinds.
const LIFECYCLE_FAULTS: &[&str] = &["process.crash", "process.restart", "process.reboot"];

/// Clock fault verbs for time-bearing kinds.
const CLOCK_FAULTS: &[&str] = &["clock.jump", "clock.drift", "clock.freeze"];

/// Durability fault verbs for persistent-state kinds.
const DURABILITY_FAULTS: &[&str] = &["durability.violate"];

/// Builtin entity kind specification.
struct BuiltinEntityKindSpec {
    /// Stable kind identifier.
    kind_id: &'static str,
    /// Semantic behavior facets for this kind.
    facets: &'static [ResourceFacet],
}

/// Builtin edge kind specification.
struct BuiltinEdgeKindSpec {
    /// Stable kind identifier.
    kind_id: &'static str,
    /// Semantic behavior facets for this kind.
    facets: &'static [ResourceFacet],
}

/// Builtin entity kind catalog.
const BUILTIN_ENTITY_KIND_SPECS: &[BuiltinEntityKindSpec] = &[
    BuiltinEntityKindSpec {
        kind_id: "runtime.instance",
        facets: &[ResourceFacet::Clock],
    },
    BuiltinEntityKindSpec {
        kind_id: "runtime.worker",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "process.instance",
        facets: &[ResourceFacet::Process],
    },
    BuiltinEntityKindSpec {
        kind_id: "time.timer",
        facets: &[ResourceFacet::Timer, ResourceFacet::Clock],
    },
    BuiltinEntityKindSpec {
        kind_id: "thread.instance",
        facets: &[ResourceFacet::Thread],
    },
    BuiltinEntityKindSpec {
        kind_id: "io.stream",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "fs.inode",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindSpec {
        kind_id: "fs.dentry",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindSpec {
        kind_id: "fs.open_file",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindSpec {
        kind_id: "fs.mount",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindSpec {
        kind_id: "fs.watch",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindSpec {
        kind_id: "net.namespace",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "net.interface",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "net.socket",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "net.listener",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "net.connection",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "net.resolver",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindSpec {
        kind_id: "process.child",
        facets: &[ResourceFacet::Process],
    },
    BuiltinEntityKindSpec {
        kind_id: "audio.device",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "audio.stream",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "input.device",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "gpu.device",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "gpu.queue",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "ipc.channel",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindSpec {
        kind_id: "device.handle",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "display.surface",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "memory.region",
        facets: &[ResourceFacet::Memory],
    },
    BuiltinEntityKindSpec {
        kind_id: "thread.worker",
        facets: &[ResourceFacet::Thread],
    },
    BuiltinEntityKindSpec {
        kind_id: "time.clock",
        facets: &[ResourceFacet::Clock],
    },
    BuiltinEntityKindSpec {
        kind_id: "tls.session",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "security.policy",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "os.service",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "random.stream",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindSpec {
        kind_id: "resource.handle",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "tty.device",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEntityKindSpec {
        kind_id: "ffi.handle",
        facets: &[],
    },
    BuiltinEntityKindSpec {
        kind_id: "crypto.key_store",
        facets: &[ResourceFacet::Storage],
    },
    BuiltinEntityKindSpec {
        kind_id: "error.channel",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEntityKindSpec {
        kind_id: "debug.channel",
        facets: &[ResourceFacet::EventSource],
    },
];

/// Builtin edge kind catalog.
const BUILTIN_EDGE_KIND_SPECS: &[BuiltinEdgeKindSpec] = &[
    BuiltinEdgeKindSpec {
        kind_id: "fs.parent_child",
        facets: &[],
    },
    BuiltinEdgeKindSpec {
        kind_id: "fs.fd_binding",
        facets: &[],
    },
    BuiltinEdgeKindSpec {
        kind_id: "fs.mount_attachment",
        facets: &[],
    },
    BuiltinEdgeKindSpec {
        kind_id: "net.network_link",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEdgeKindSpec {
        kind_id: "net.stream_link",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEdgeKindSpec {
        kind_id: "net.route",
        facets: &[ResourceFacet::Stream],
    },
    BuiltinEdgeKindSpec {
        kind_id: "runtime.instance.owns.worker",
        facets: &[],
    },
    BuiltinEdgeKindSpec {
        kind_id: "runtime.worker.owns.resource",
        facets: &[],
    },
    BuiltinEdgeKindSpec {
        kind_id: "ipc.channel",
        facets: &[ResourceFacet::EventSource],
    },
    BuiltinEdgeKindSpec {
        kind_id: "process.pipe",
        facets: &[ResourceFacet::Stream],
    },
];

/// Return builtin entity kind definitions.
pub(super) fn builtin_entity_kinds() -> Vec<WorldEntityKindDefinition> {
    BUILTIN_ENTITY_KIND_SPECS
        .iter()
        .map(|spec| {
            WorldEntityKindDefinition::new(spec.kind_id)
                .labels(labels_for_kind(spec.kind_id))
                .supports_faults(faults_for_facets(BASE_ENTITY_FAULTS, spec.facets))
        })
        .collect()
}

/// Return builtin resource entity kind definitions.
pub(super) fn builtin_resource_entity_kinds() -> Vec<WorldEntityKindDefinition> {
    ResourceKind::all()
        .iter()
        .map(|resource_kind| {
            let kind_id = resource_kind.kind_id();
            WorldEntityKindDefinition::new(kind_id)
                .labels(labels_for_kind(kind_id))
                .supports_faults(resource_faults(*resource_kind))
        })
        .collect()
}

/// Return builtin edge kind definitions.
pub(super) fn builtin_edge_kinds() -> Vec<WorldEdgeKindDefinition> {
    BUILTIN_EDGE_KIND_SPECS
        .iter()
        .map(|spec| {
            WorldEdgeKindDefinition::new(spec.kind_id)
                .labels(labels_for_kind(spec.kind_id))
                .supports_faults(faults_for_facets(BASE_EDGE_FAULTS, spec.facets))
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
    labels.insert(LABEL_TOPOLOGY_KIND.to_string(), kind_id.to_string());

    labels
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        BUILTIN_EDGE_KIND_SPECS, BUILTIN_ENTITY_KIND_SPECS, builtin_edge_kinds,
        builtin_entity_kinds, builtin_resource_entity_kinds, resource_faults,
    };
    use crate::platform::ResourceKind;

    /// Ensure builtin entity kind specs stay unique.
    #[test]
    fn test_builtin_entity_kind_specs_are_unique() {
        // collect all builtin entity kind ids
        let kind_ids = BUILTIN_ENTITY_KIND_SPECS
            .iter()
            .map(|spec| spec.kind_id)
            .collect::<Vec<_>>();
        let unique_kind_ids = kind_ids.iter().copied().collect::<BTreeSet<_>>();

        // every declared builtin entity kind should be unique
        assert_eq!(kind_ids.len(), unique_kind_ids.len());
        assert_eq!(builtin_entity_kinds().len(), unique_kind_ids.len());
    }

    /// Ensure builtin edge kind specs stay unique.
    #[test]
    fn test_builtin_edge_kind_specs_are_unique() {
        // collect all builtin edge kind ids
        let kind_ids = BUILTIN_EDGE_KIND_SPECS
            .iter()
            .map(|spec| spec.kind_id)
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

        assert!(socket_faults.contains("transport.drop"));
        assert!(file_faults.contains("durability.violate"));
        assert!(timer_faults.contains("clock.jump"));
        assert!(process_faults.contains("process.crash"));
        assert!(!shared_memory_faults.contains("durability.violate"));
        assert!(!poll_faults.contains("transport.drop"));
        assert!(watch_faults.contains("transport.drop"));
    }
}
