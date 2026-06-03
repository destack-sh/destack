use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::{
    ModuleQueryContext, QueryModule, QueryPosition, QueryTarget, WorkspaceQueryContext,
};
use crate::dir::SymbolReferenceSearch;
use destack_dir::GlobalSymbolId;

/// Role of one reference occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceRole {
    /// Declaration occurrence.
    Declaration,
    /// Read occurrence.
    Read,
    /// Write occurrence.
    Write,
    /// Type occurrence.
    Type,
    /// Import occurrence.
    Import,
    /// Export occurrence.
    Export,
}

/// One symbol reference occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// The referenced source target.
    pub target: QueryTarget,
    /// The reference role.
    pub role: ReferenceRole,
}

/// Request find references at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindReferencesRequest {
    /// The queried position.
    pub position: QueryPosition,
    /// Whether to include the declaration in results.
    pub include_declaration: bool,
}

/// Response payload for find references queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindReferencesResponse {
    /// Reference occurrences.
    pub references: Vec<Reference>,
}

/// Remove overlapping spans by keeping the most specific span at each overlap.
fn prune_overlapping_spans(references: &mut Vec<(QueryModule, Span, ReferenceRole)>) {
    if references.len() < 2 {
        return;
    }

    let mut filtered = Vec::with_capacity(references.len());
    for reference in references.iter().copied() {
        let Some(last_reference) = filtered.last_mut() else {
            filtered.push(reference);
            continue;
        };
        let span = reference.1;
        let last_span = &mut last_reference.1;

        if !last_span.intersects(span) {
            filtered.push(reference);
            continue;
        }

        if span.len() < last_span.len()
            || (span.len() == last_span.len() && span.start >= last_span.start)
        {
            *last_reference = reference;
        }
    }

    *references = filtered;
}

impl ModuleQueryContext<'_> {
    /// Find all references to the symbol at the given position.
    ///
    /// Optionally includes the declaration in the results.
    pub fn find_references(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        offset: u32,
        include_declaration: bool,
    ) -> Vec<Reference> {
        let ctx = self;
        // find the symbol at offset
        let Some(symbol_at) = ctx.find_symbol_at_offset(offset) else {
            return Vec::new();
        };

        // preserve local import aliases as local reference targets
        let (target_symbol, declaration_span, target_name) =
            if let Some(local_alias_name) = ctx.local_import_alias_name(symbol_at.symbol_id) {
                let declaration_span = include_declaration
                    .then(|| {
                        ctx.symbol_local_definition_span(symbol_at.symbol_id)
                            .or_else(|| ctx.symbol_definition_span(symbol_at.symbol_id))
                    })
                    .flatten();

                (
                    symbol_at.symbol_id,
                    declaration_span,
                    Some(local_alias_name),
                )
            } else {
                let canonical_id = ctx.canonical_symbol(symbol_at.symbol_id);
                let canonical_name = ctx.symbol_name(canonical_id);
                let declaration_span = include_declaration
                    .then(|| {
                        ctx.symbol_definition_span(canonical_id).or_else(|| {
                            let target_ctx = ctx.module_context(canonical_id.module_id)?;
                            target_ctx.symbol_local_definition_span(canonical_id)
                        })
                    })
                    .flatten();

                (canonical_id, declaration_span, canonical_name)
            };

        // search all modules for references to that symbol
        let spans = ctx.find_references_to_symbol(
            workspace,
            target_symbol,
            declaration_span,
            target_name.as_deref(),
        );
        spans
            .into_iter()
            .map(|(module, span, role)| Reference {
                target: QueryTarget::span(module, span).with_symbol(target_symbol),
                role,
            })
            .collect()
    }

    /// Find all references to a symbol across all modules.
    fn find_references_to_symbol(
        &self,
        workspace: &WorkspaceQueryContext<'_>,
        canonical_id: GlobalSymbolId,
        declaration_span: Option<Span>,
        target_name: Option<&str>,
    ) -> Vec<(QueryModule, Span, ReferenceRole)> {
        let ctx = self;
        // initialize the reference list
        let mut references = Vec::new();

        // configure reference collection for find references behavior
        let reference_search = SymbolReferenceSearch {
            include_expressions: true,
            include_members: true,
            include_dependency_items: true,
            include_namespace_receivers: true,
            skip_dependency_aliases: false,
            use_dependency_name_spans: true,
            target_name,
            require_target_name_match: false,
            limit_file: None,
        };

        // collect references across candidate modules only
        for module_id in workspace.modules_referencing_symbol(canonical_id) {
            let Some(module_ctx) = ctx.module_context(module_id) else {
                continue;
            };

            // collect and append references for this module
            let spans = module_ctx
                .dir()
                .symbol_references(canonical_id, reference_search);
            let module = module_ctx.query_module();
            references.extend(
                spans
                    .into_iter()
                    .map(|span| (module, span, ReferenceRole::Read)),
            );
        }

        // normalize ordering and remove duplicates
        references.sort_by_key(|(_, span, _)| (span.file, span.start, span.end));
        references.dedup();
        prune_overlapping_spans(&mut references);
        ctx.sort_reference_spans(&mut references);

        // place the declaration first when requested
        if let Some(decl_span) = declaration_span {
            references.retain(|(_, span, _)| {
                !(span.file == decl_span.file
                    && span.start == decl_span.start
                    && span.end == decl_span.end)
            });
            references.insert(
                0,
                (ctx.query_module(), decl_span, ReferenceRole::Declaration),
            );
        }

        // return the final reference list
        references
    }

    /// Sort reference spans by stable file location.
    fn sort_reference_spans(&self, references: &mut [(QueryModule, Span, ReferenceRole)]) {
        let ctx = self;
        references.sort_by(|left, right| {
            let left_key = ctx.reference_span_sort_key(left.1);
            let right_key = ctx.reference_span_sort_key(right.1);

            left_key.cmp(&right_key)
        });
    }

    /// Build a stable sort key for a reference span.
    fn reference_span_sort_key(&self, span: Span) -> (String, u32, u32, u128) {
        let ctx = self;
        let Some(file) = ctx
            .repository()
            .file(ctx.revision(), span.file)
            .ok()
            .flatten()
        else {
            return (String::new(), span.start, span.end, span.file.0);
        };

        // prefer the displayed file name used by query snapshots
        let file_key = if !file.name.is_empty() {
            file.name.clone()
        }
        // otherwise compare canonical paths
        else if let Some(path) = file.path.as_ref() {
            path.to_string_lossy().to_string()
        }
        // otherwise compare uri strings
        else {
            file.uri.to_string()
        };

        (file_key, span.start, span.end, span.file.0)
    }
}
