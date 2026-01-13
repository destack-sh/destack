/// External resource table and finalizer registry.
#[derive(Debug, Default)]
pub struct Resources;

/// Identifier for a runtime-managed resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);
