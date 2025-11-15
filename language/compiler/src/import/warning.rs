use dyst_dir::ModuleId;

/// Warning when importing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ImportWarning {
    /// Missing configuration for a file.
    MissingConfiguration { module: ModuleId } = 1,
}

impl ImportWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::MissingConfiguration { .. } => 1,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self) -> &'static str {
        match self {
            Self::MissingConfiguration { .. } => "missing configuration for a module",
        }
    }
}

impl std::fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportWarning")
            .field("code", &format!("LW{:03}", self.sub_code()))
            .finish()
    }
}

