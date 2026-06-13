use destack_source::{ComponentId, ModuleId};

/// Artifact coordinates for one checked component.
/// TODO #Cleanup: move CheckComponentKey to check/state or something
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct CheckComponentKey {
    /// The checked component entry module.
    pub entry: ModuleId,
    /// The checked component id.
    pub component: ComponentId,
}
