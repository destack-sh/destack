use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    ReferenceCollectionOptions, collect_symbol_references_in_context, find_symbol_at_offset,
    get_canonical_symbol, get_symbol_definition_span, sort_and_dedup_spans,
};
use destack_dir::GlobalSymbolId;

/// Result of a find references query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

/// Request find references at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindReferencesRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// Whether to include the declaration in results.
    pub include_declaration: bool,
}

/// Response payload for find references queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindReferencesResponse {
    /// References result, if any.
    pub result: Option<ReferencesResult>,
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
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // get canonical symbol and resolve imports
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // search all modules for references to that symbol
    let references = find_references_to_symbol(session, canonical_id, include_declaration);

    Some(ReferencesResult {
        references,
        include_declaration,
    })
}

/// Find all references to a symbol across all modules.
fn find_references_to_symbol(
    session: &Session,
    canonical_id: GlobalSymbolId,
    include_declaration: bool,
) -> Vec<Span> {
    // initialize the reference list
    let mut references = Vec::new();

    // resolve the declaration span when requested
    let declaration_span = if include_declaration {
        get_symbol_definition_span(session, canonical_id)
    } else {
        None
    };

    // configure reference collection for find references behavior
    let reference_options = ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_members: false,
        skip_dependency_aliases: false,
        use_dependency_name_spans: false,
        target_name: None,
        limit_to_file: None,
    };

    // collect references across all modules
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };

        // collect and append references for this module
        let spans =
            collect_symbol_references_in_context(session, &ctx, canonical_id, reference_options);
        references.extend(spans);
    }

    // normalize ordering and remove duplicates
    sort_and_dedup_spans(&mut references);

    // place the declaration first when requested
    if let Some(decl_span) = declaration_span {
        references.retain(|span| {
            !(span.file == decl_span.file
                && span.start == decl_span.start
                && span.end == decl_span.end)
        });
        references.insert(0, decl_span);
    }

    // return the final reference list
    references
}
