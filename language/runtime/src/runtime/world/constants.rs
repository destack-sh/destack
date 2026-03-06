/// Builtin runtime node kind id.
pub const BUILTIN_RUNTIME_KIND_ID: &str = "runtime.instance";
/// Builtin agent node kind id.
pub const BUILTIN_AGENT_KIND_ID: &str = "runtime.agent";
/// Builtin runtime-to-agent edge kind id.
pub const BUILTIN_RUNTIME_OWNS_AGENT_EDGE_KIND_ID: &str = "runtime.instance.owns.agent";
/// Builtin agent-to-resource edge kind id.
pub const BUILTIN_AGENT_OWNS_RESOURCE_EDGE_KIND_ID: &str = "runtime.agent.owns.resource";
/// System label key that stores one topology kind id.
pub const LABEL_TOPOLOGY_KIND: &str = "topology.kind";
/// System label key that stores one runtime display name.
pub const LABEL_RUNTIME_NAME: &str = "runtime.name";
/// System label key that stores one agent display name.
pub const LABEL_AGENT_NAME: &str = "agent.name";
/// System label key that stores one resource display label.
pub const LABEL_RESOURCE_LABEL: &str = "resource.label";
/// First allocated runtime id.
pub(crate) const INITIAL_RUNTIME_ID: u64 = 1;
/// First allocated agent id.
pub(crate) const INITIAL_AGENT_ID: u64 = 1;
/// First command revision id.
pub(crate) const INITIAL_CONTROL_REVISION: u64 = 1;
