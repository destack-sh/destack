use destack_dir::SymbolType;
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_definition_span,
};

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
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<ImplementationResult> {
    // 1. find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // 2. get canonical symbol (resolve imports)
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // 3. check if it's an interface or class (types that can be implemented/extended)
    let target_module = session.modules.get(canonical_id.module_id);
    let target_module = target_module.read();
    let Some(target_dir) = &target_module.dir else {
        return Some(ImplementationResult::empty());
    };
    let symbols = target_dir.symbols.read();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    let is_interface = symbol.ty == SymbolType::Interface;
    let is_class = symbol.ty == SymbolType::Class;

    if !is_interface && !is_class {
        // not something that can be implemented/extended
        return Some(ImplementationResult::empty());
    }

    // 4. search all modules for types that implement/extend this symbol
    let mut locations = Vec::new();
    for module in session.modules.iter() {
        let module = module.read();
        let Some(dir) = &module.dir else {
            continue;
        };
        let types = dir.types.read();

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

    Some(ImplementationResult { locations })
}
