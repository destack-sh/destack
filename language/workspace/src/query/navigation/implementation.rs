use destack_dir::SymbolType;
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_definition_span, sort_and_dedup_spans,
};

/// Result of a goto implementation query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

/// Request goto implementation at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoImplementationRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto implementation queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoImplementationResponse {
    /// Implementation locations, if any.
    pub result: Option<ImplementationResult>,
}

/// Find implementations of the symbol at the given position.
///
/// For interfaces: finds implementing structs/classes.
/// For abstract methods: finds concrete implementations.
/// For classes: finds subclasses.
pub fn goto_implementation(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<ImplementationResult> {
    // find the symbol at the cursor position
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // normalize through canonical symbols and imports
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // resolve the target symbol type information
    let target_module = session.modules.get(canonical_id.module_id);
    let target_module = target_module.read();
    let Some(ctx) = session.query_context(&target_module) else {
        return Some(ImplementationResult::empty());
    };

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    let is_interface = symbol.ty == SymbolType::Interface;
    let is_class = symbol.ty == SymbolType::Class;

    if !is_interface && !is_class {
        return Some(ImplementationResult::empty());
    }

    drop(symbols);
    drop(target_module);

    // search all modules for types that implement or extend this symbol
    let mut locations = Vec::new();
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let types = ctx.types();

        // check all lineages in this module
        for (symbol_id, lineage) in types.iter_lineages() {
            // check if this type implements the interface
            let matches = if is_interface {
                lineage.directly_implements(canonical_id)
            } else {
                // for classes, check extends
                lineage.directly_extends(canonical_id)
            };

            if matches {
                // get the span of the implementing type's declaration
                if let Some(span) = get_symbol_definition_span(session, symbol_id) {
                    locations.push(span);
                }
            }
        }
    }

    // normalize spans for stable ordering and deduplication
    sort_and_dedup_spans(&mut locations);

    Some(ImplementationResult { locations })
}
