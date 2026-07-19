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
        Self::validate_id(id);

        Self {
            id,
            description,
            severity: DiagnosticSeverity::Error,
            is_controllable: false,
        }
    }

    /// Define one fixed warning diagnostic.
    pub const fn warning(id: &'static str, description: &'static str) -> Self {
        Self::validate_id(id);

        Self {
            id,
            description,
            severity: DiagnosticSeverity::Warning,
            is_controllable: false,
        }
    }

    /// Define one warning selectable by source controls.
    pub const fn controllable_warning(id: &'static str, description: &'static str) -> Self {
        Self::validate_id(id);

        Self {
            id,
            description,
            severity: DiagnosticSeverity::Warning,
            is_controllable: true,
        }
    }

    /// Require one nonempty kebab-case diagnostic id.
    const fn validate_id(id: &str) {
        let bytes = id.as_bytes();
        assert!(!bytes.is_empty(), "empty diagnostic id");

        // validate every byte and segment boundary
        let mut index = 0;
        let mut segment_is_empty = true;
        while index < bytes.len() {
            let byte = bytes[index];

            // begin the next segment after a completed segment
            if byte == b'-' {
                assert!(!segment_is_empty, "empty diagnostic id segment");
                segment_is_empty = true;
            }
            // accept lowercase ASCII letters and digits
            else {
                let is_lowercase = byte >= b'a' && byte <= b'z';
                let is_digit = byte >= b'0' && byte <= b'9';
                assert!(is_lowercase || is_digit, "invalid diagnostic id character");
                segment_is_empty = false;
            }
            index += 1;
        }

        // reject a trailing separator
        assert!(!segment_is_empty, "empty diagnostic id segment");
    }
}
