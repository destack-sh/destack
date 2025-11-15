use dyst_dir::{Expression, ModuleId, NodeId};

/// Warning when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResolveWarning {
    /// Unknown import.
    UnknownImport {
        module: ModuleId,
        node: NodeId<Expression>,
    } = 1,
}

impl ResolveWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnknownImport { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnknownImport { .. } => "unknown import",
        }
    }
}

impl std::fmt::Display for ResolveWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolveWarning")
            .field("code", &format!("XE{:03}", self.sub_code()))
            .finish()
    }
}
