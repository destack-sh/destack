use destack_source::{FileId, Span};

use crate::Session;

/// Result of a goto definition query.
#[derive(Debug, Clone, Default)]
pub struct DefinitionResult {
    /// The definition location(s).
    /// Multiple locations for overloaded symbols or partial definitions.
    pub locations: Vec<Span>,
}

impl DefinitionResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            locations: Vec::new(),
        }
    }

    /// Create a result with a single span.
    pub fn single(span: Span) -> Self {
        Self {
            locations: vec![span],
        }
    }

    /// Whether any definitions were found.
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
}

/// Find the definition of the symbol at the given position.
///
/// Returns the location(s) where the symbol is defined.
/// For imports, follows to the original definition.
pub fn goto_definition(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<DefinitionResult> {
    // 1. find the token/node at offset
    // 2. get the symbol for that node
    // 3. follow target_symbol chain to canonical definition
    // 4. get the primary_declaration span
    // 5. return location
    todo!("#Incomplete: goto_definition")
}

/// Find the declaration of the symbol at the given position.
///
/// For imports, returns the import statement location.
/// For locals, same as goto_definition.
pub fn goto_declaration(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<DefinitionResult> {
    // similar to goto_definition but doesn't follow imports
    todo!("#Incomplete: goto_declaration")
}

/// Find the type definition of the symbol at the given position.
///
/// For a variable, returns the location of its type's definition.
/// For a type, returns the type itself.
pub fn goto_type_definition(
    _session: &Session,
    _file: FileId,
    _offset: u32,
) -> Option<DefinitionResult> {
    // 1. find the symbol at offset
    // 2. get its type
    // 3. find the type's definition location
    todo!("#Incomplete: goto_type_definition")
}
