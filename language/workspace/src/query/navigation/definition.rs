use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::{find_symbol_at_offset, get_dir_node_span, get_symbol_definition_span};

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
pub fn goto_definition(session: &Session, file: FileId, offset: u32) -> Option<DefinitionResult> {
    // find the symbol at the offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // get the definition span
    let span = get_symbol_definition_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Find the declaration of the symbol at the given position.
///
/// For imports, returns the import statement location.
/// For locals, same as goto_definition.
pub fn goto_declaration(session: &Session, file: FileId, offset: u32) -> Option<DefinitionResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // get the declaration span
    let span = get_declaration_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Get the declaration span without following canonical_symbol.
fn get_declaration_span(session: &Session, symbol_id: destack_dir::GlobalSymbolId) -> Option<Span> {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();

    let symbols = module.dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // get primary_declaration directly
    let declaration = symbol.primary_declaration?;

    drop(symbols);

    get_dir_node_span(&module, declaration.local_id)
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
    // TODO: implement type definition lookup
    None
}
