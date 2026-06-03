use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::host::ResourceKind;

use super::{EdgeDefinition, EdgeKind, EntityDefinition, EntityKind};

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

/// Builtin entity kind definitions by kind id.
pub(super) static BUILTIN_ENTITY_KINDS: LazyLock<BTreeMap<EntityKind, EntityDefinition>> =
    LazyLock::new(|| {
        builtin_entity_kinds()
            .into_iter()
            .chain(builtin_resource_entity_kinds())
            .map(|definition| (definition.kind.clone(), definition))
            .collect()
    });

/// Builtin edge kind definitions by kind id.
pub(super) static BUILTIN_EDGE_KINDS: LazyLock<BTreeMap<EdgeKind, EdgeDefinition>> =
    LazyLock::new(|| {
        builtin_edge_kinds()
            .into_iter()
            .map(|definition| (definition.kind.clone(), definition))
            .collect()
    });

/// Return builtin entity kind definitions.
pub(super) fn builtin_entity_kinds() -> Vec<EntityDefinition> {
    BUILTIN_ENTITY_KIND_DESCRIPTORS
        .iter()
        .map(|descriptor| {
            EntityDefinition::new(descriptor.kind_id).labels(labels_for_kind(descriptor.kind_id))
        })
        .collect()
}

/// Return builtin resource entity kind definitions.
pub(super) fn builtin_resource_entity_kinds() -> Vec<EntityDefinition> {
    ResourceKind::all()
        .iter()
        .map(|resource_kind| {
            let kind_id = resource_kind.kind_id();
            EntityDefinition::new(kind_id).labels(labels_for_kind(kind_id))
        })
        .collect()
}

/// Return builtin edge kind definitions.
pub(super) fn builtin_edge_kinds() -> Vec<EdgeDefinition> {
    BUILTIN_EDGE_KIND_DESCRIPTORS
        .iter()
        .map(|descriptor| {
            EdgeDefinition::new(descriptor.kind_id).labels(labels_for_kind(descriptor.kind_id))
        })
        .collect()
}

/// Return whether one entity kind id is builtin.
pub(super) fn is_builtin_entity_kind(kind: &str) -> bool {
    BUILTIN_ENTITY_KIND_DESCRIPTORS
        .iter()
        .any(|descriptor| descriptor.kind_id == kind)
        || ResourceKind::all()
            .iter()
            .any(|resource_kind| resource_kind.kind_id() == kind)
}

/// Return whether one edge kind id is builtin.
pub(super) fn is_builtin_edge_kind(kind: &str) -> bool {
    BUILTIN_EDGE_KIND_DESCRIPTORS
        .iter()
        .any(|descriptor| descriptor.kind_id == kind)
}

/// Build system labels for one kind id.
fn labels_for_kind(kind_id: &str) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(EntityKind::LABEL_KIND.to_string(), kind_id.to_string());

    labels
}
