/// Builtin runtime entity kind id.
pub const BUILTIN_RUNTIME_KIND_ID: &str = "runtime.instance";
/// Builtin worker entity kind id.
pub const BUILTIN_WORKER_KIND_ID: &str = "runtime.worker";
/// Builtin runtime-to-worker edge kind id.
pub const BUILTIN_RUNTIME_OWNS_WORKER_EDGE_KIND_ID: &str = "runtime.instance.owns.worker";
/// Builtin worker-to-resource edge kind id.
pub const BUILTIN_WORKER_OWNS_RESOURCE_EDGE_KIND_ID: &str = "runtime.worker.owns.resource";
/// System label key that stores one topology kind id.
pub const LABEL_TOPOLOGY_KIND: &str = "topology.kind";
/// System label key that stores one runtime display name.
pub const LABEL_RUNTIME_NAME: &str = "runtime.name";
/// System label key that stores one worker display name.
pub const LABEL_WORKER_NAME: &str = "worker.name";
/// System label key that stores one resource display label.
pub const LABEL_RESOURCE_LABEL: &str = "resource.label";
