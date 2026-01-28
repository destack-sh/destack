use destack_dir::{Declaration, GlobalSymbolId, SymbolType};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_definition_span,
    resolve_nominal_symbol_from_type_expression, sort_and_dedup_spans,
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

    // resolve the target symbol metadata
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    // classify the symbol by type
    let is_interface = symbol.ty == SymbolType::Interface;
    let is_class = symbol.ty == SymbolType::Class;

    // bail out for symbols that cannot be implemented
    if !is_interface && !is_class {
        return Some(ImplementationResult::empty());
    }

    // release the target module handles before scanning
    drop(symbols);
    drop(target_module);

    // initialize the result spans
    let mut locations = Vec::new();

    // search all modules for types that implement or extend this symbol
    for module in session.modules.iter() {
        // resolve query context for each module
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };

        // check all lineages in this module
        {
            let types = ctx.types();
            for (symbol_id, lineage) in types.iter_lineages() {
                // check if this type implements or extends the target symbol
                let matches = if is_interface {
                    lineage
                        .implements
                        .iter()
                        .any(|symbol| get_canonical_symbol(session, *symbol) == canonical_id)
                } else {
                    lineage
                        .extends
                        .map(|symbol| get_canonical_symbol(session, symbol) == canonical_id)
                        .unwrap_or(false)
                };

                if matches {
                    // get the span of the implementing type's declaration
                    if let Some(span) = get_symbol_definition_span(session, symbol_id) {
                        locations.push(span);
                    }
                }
            }
        }

        // scan syntactic heritage when lineages are incomplete
        let dir_tree = ctx.tree();
        for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // extract declaration heritage and symbol
            let (descriptor, heritage) = match declaration {
                Declaration::Struct {
                    descriptor,
                    heritage,
                    ..
                }
                | Declaration::Class {
                    descriptor,
                    heritage,
                    ..
                } => (descriptor, heritage),
                _ => continue,
            };

            // select the relevant heritage clause
            let related_types = if is_interface {
                heritage.implements_types.as_ref()
            } else {
                heritage.extends_types.as_ref()
            };
            let Some(related_types) = related_types else {
                continue;
            };

            // resolve heritage types to nominal symbols
            let mut matches = false;
            for type_expr_id in related_types {
                let Some(symbol_id) =
                    resolve_nominal_symbol_from_type_expression(session, &ctx, *type_expr_id)
                else {
                    continue;
                };

                // record when a related type matches the target
                if get_canonical_symbol(session, symbol_id) == canonical_id {
                    matches = true;
                    break;
                }
            }

            // skip unrelated declarations
            if !matches {
                continue;
            }

            // record the implementing declaration span
            let symbol_id = GlobalSymbolId {
                module_id: ctx.module_id,
                local_id: descriptor.symbol,
            };
            if let Some(span) = get_symbol_definition_span(session, symbol_id) {
                locations.push(span);
            }
        }
    }

    // normalize spans for stable ordering and deduplication
    sort_and_dedup_spans(&mut locations);

    Some(ImplementationResult { locations })
}
