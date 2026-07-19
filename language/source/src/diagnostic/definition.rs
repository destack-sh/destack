use crate::DiagnosticSeverity;

/// Static metadata for one diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticDefinition {
    /// The canonical diagnostic id.
    pub id: &'static str,
    /// The diagnostic description.
    pub description: &'static str,
    /// The diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Whether source controls may select the diagnostic.
    pub is_controllable: bool,
}

impl DiagnosticDefinition {
    /// Define one error diagnostic.
    pub const fn error(id: &'static str, description: &'static str) -> Self {
        assert!(!id.is_empty(), "empty diagnostic id");

        Self {
            id,
            description,
            severity: DiagnosticSeverity::Error,
            is_controllable: false,
        }
    }

    /// Define one fixed warning diagnostic.
    pub const fn warning(id: &'static str, description: &'static str) -> Self {
        assert!(!id.is_empty(), "empty diagnostic id");

        Self {
            id,
            description,
            severity: DiagnosticSeverity::Warning,
            is_controllable: false,
        }
    }

    /// Define one warning selectable by source controls.
    pub const fn controllable_warning(id: &'static str, description: &'static str) -> Self {
        assert!(!id.is_empty(), "empty diagnostic id");

        Self {
            id,
            description,
            severity: DiagnosticSeverity::Warning,
            is_controllable: true,
        }
    }
}
