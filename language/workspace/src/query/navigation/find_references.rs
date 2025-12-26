use destack_dir::Expression;
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_dir_node_span, get_symbol_definition_span,
};

/// Result of a find references query.
#[derive(Debug, Clone, Default)]
pub struct ReferencesResult {
    /// All reference locations.
    pub references: Vec<Span>,
    /// Whether the definition is included in the results.
    pub include_declaration: bool,
}

impl ReferencesResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            references: Vec::new(),
            include_declaration: false,
        }
    }

    /// Whether any references were found.
    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    /// Number of references found.
    pub fn len(&self) -> usize {
        self.references.len()
    }
}

/// Find all references to the symbol at the given position.
///
/// Optionally includes the declaration in the results.
pub fn find_references(
    session: &Session,
    file: FileId,
    offset: u32,
    include_declaration: bool,
) -> Option<ReferencesResult> {
    // 1. find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // 2. get canonical symbol (resolve imports)
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // 3. search all modules for references to that symbol
    let mut references = Vec::new();

    for module in session.modules.iter() {
        let module = module.read();
        let Some(ast) = &module.ast else {
            continue;
        };
        let profile = session.default_profile_for_module(module.id);
        let Some(dir) = module.dir_maybe(profile) else {
            continue;
        };

        // collect matching expression ids
        let matching_expr_ids: Vec<_> = {
            let dir_tree = dir.tree.read();
            dir_tree
                .iter_nodes_of_type::<Expression>()
                .filter_map(|(expr_id, expr)| {
                    if let Some(target) = expr.target_symbol() {
                        let target_canonical = get_canonical_symbol(session, target);
                        if target_canonical == canonical_id {
                            return Some(expr_id);
                        }
                    }
                    None
                })
                .collect()
        };

        // get spans for each matching expression
        for expr_id in matching_expr_ids {
            if let Some(span) = get_dir_node_span(ast, dir, expr_id.into()) {
                references.push(span);
            }
        }
    }

    // 4. optionally include the declaration itself
    if include_declaration
        && let Some(decl_span) = get_symbol_definition_span(session, canonical_id)
    {
        // insert at the beginning so declaration comes first
        references.insert(0, decl_span);
    }

    Some(ReferencesResult {
        references,
        include_declaration,
    })
}
