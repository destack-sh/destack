use dyst_dir::{Expression, NodeId};

/// Warning when optimizing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum OptimizeWarning {
    /// Unknown type for an expression.
    MissingType { node: NodeId<Expression> } = 1,
}

impl OptimizeWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingType { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> String {
        match self {
            Self::MissingType { .. } => "unknown type for an expression".to_string(),
        }
    }
}

impl std::fmt::Display for OptimizeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeWarning")
            .field("code", &format!("OW{:03}", self.sub_code()))
            .finish()
    }
}
