use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::ast::sort_and_dedup_spans;
use crate::core::{RepositoryQueryIndexExt, query_context};
use crate::dir::{
    ReferenceCollectionOptions, collect_symbol_references_in_context, find_symbol_at_offset,
    get_canonical_symbol, get_symbol_definition_span, get_symbol_local_definition_span,
    resolve_local_import_alias_name, resolve_symbol_name,
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
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
    include_declaration: bool,
) -> Option<ReferencesResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;

    // preserve local import aliases as local reference targets
    let (target_symbol, declaration_span, target_name) = if let Some(local_alias_name) =
        resolve_local_import_alias_name(repository, revision, symbol_at.symbol_id)
    {
        let declaration_span = include_declaration
            .then(|| {
                get_symbol_local_definition_span(repository, revision, symbol_at.symbol_id).or_else(
                    || get_symbol_definition_span(repository, revision, symbol_at.symbol_id),
                )
            })
            .flatten();

        (
            symbol_at.symbol_id,
            declaration_span,
            Some(local_alias_name),
        )
    } else {
        let canonical_id = get_canonical_symbol(repository, revision, symbol_at.symbol_id);
        let canonical_name = resolve_symbol_name(repository, revision, canonical_id);
        let declaration_span = include_declaration
            .then(|| {
                get_symbol_definition_span(repository, revision, canonical_id).or_else(|| {
                    get_symbol_local_definition_span(repository, revision, canonical_id)
                })
            })
            .flatten();

        (canonical_id, declaration_span, canonical_name)
    };

    // search all modules for references to that symbol
    let references = find_references_to_symbol(
        repository,
        revision,
        target_symbol,
        declaration_span,
        target_name.as_deref(),
    );

    Some(ReferencesResult {
        references,
        include_declaration,
    })
}

/// Find all references to a symbol across all modules.
fn find_references_to_symbol(
    repository: &Repository,
    revision: Revision,
    canonical_id: GlobalSymbolId,
    declaration_span: Option<Span>,
    target_name: Option<&str>,
) -> Vec<Span> {
    // initialize the reference list
    let mut references = Vec::new();

    // configure reference collection for find references behavior
    let reference_options = ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_receivers: true,
        skip_dependency_aliases: false,
        use_dependency_name_spans: true,
        target_name,
        require_target_name_match: false,
        limit_to_file: None,
    };

    // collect references across candidate modules only
    for module_id in repository.reference_index_modules_for_target(revision, canonical_id) {
        let Some(ctx) = query_context(repository, revision, module_id) else {
            continue;
        };

        // collect and append references for this module
        let spans = collect_symbol_references_in_context(
            repository,
            ctx.ast(),
            ctx.dir(),
            canonical_id,
            reference_options,
        );
        references.extend(spans);
    }

    // normalize ordering and remove duplicates
    sort_and_dedup_spans(&mut references);
    prune_overlapping_spans(&mut references);
    sort_reference_spans(repository, revision, &mut references);

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

/// Remove overlapping spans by keeping the most specific span at each overlap.
fn prune_overlapping_spans(spans: &mut Vec<Span>) {
    if spans.len() < 2 {
        return;
    }

    let mut filtered = Vec::with_capacity(spans.len());
    for span in spans.iter().copied() {
        let Some(last_span) = filtered.last_mut() else {
            filtered.push(span);
            continue;
        };

        if !last_span.intersects(span) {
            filtered.push(span);
            continue;
        }

        if span.len() < last_span.len()
            || (span.len() == last_span.len() && span.start >= last_span.start)
        {
            *last_span = span;
        }
    }

    *spans = filtered;
}

/// Sort reference spans by stable file location.
fn sort_reference_spans(repository: &Repository, revision: Revision, spans: &mut [Span]) {
    spans.sort_by(|left, right| {
        let left_key = reference_span_sort_key(repository, revision, *left);
        let right_key = reference_span_sort_key(repository, revision, *right);

        left_key.cmp(&right_key)
    });
}

/// Build a stable sort key for a reference span.
fn reference_span_sort_key(
    repository: &Repository,
    revision: Revision,
    span: Span,
) -> (String, u32, u32, u128) {
    let Some(file) = repository.file(revision, span.file).ok().flatten() else {
        return (String::new(), span.start, span.end, span.file.0);
    };

    // prefer the displayed file name used by query snapshots
    let file_key = if !file.name.is_empty() {
        file.name.clone()
    }
    // otherwise fall back to canonical paths
    else if let Some(path) = file.path.as_ref() {
        path.to_string_lossy().to_string()
    }
    // otherwise fall back to the uri string
    else {
        file.uri.to_string()
    };

    (file_key, span.start, span.end, span.file.0)
}
