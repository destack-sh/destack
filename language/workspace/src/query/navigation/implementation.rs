use destack_source::{FileId, Span};

use crate::Session;

/// Result of a goto implementation query.
#[derive(Debug, Clone, Default)]
pub struct ImplementationResult {
    /// Implementation locations.
    pub locations: Vec<Span>,
}

impl ImplementationResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            locations: Vec::new(),
        }
    }

    /// Whether any implementations were found.
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
}

/// Find implementations of the symbol at the given position.
///
/// For interfaces: finds implementing structs/classes.
/// For abstract methods: finds concrete implementations.
/// For classes: finds subclasses.
pub fn goto_implementation(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<ImplementationResult> {
    // 1. find the symbol at offset
    // 2. if interface, find all types that implement it
    // 3. if abstract method, find all implementations
    // 4. if class, find all subclasses
    todo!("#Incomplete: goto_implementation")
}
