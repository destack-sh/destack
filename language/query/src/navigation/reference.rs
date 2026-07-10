use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::{Module, ModuleQueryContext, Position, ProgramQueryContext, ReferenceFilter, Target};

/// Role of one reference occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Reference {
    /// The referenced source target.
    pub target: Target,
    /// The reference role.
    pub role: ReferenceRole,
}

/// One internal reference occurrence before protocol shaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReferenceOccurrence {
    /// The module containing the reference.
    module: Module,
    /// The reference source span.
    span: Span,
    /// The reference role.
    role: ReferenceRole,
}

impl ReferenceOccurrence {
    /// Create one reference occurrence.
    fn new(module: Module, span: Span, role: ReferenceRole) -> Self {
        Self { module, span, role }
    }

    /// Remove overlapping spans by keeping the most specific span at each overlap.
    fn prune_overlaps(references: &mut Vec<Self>) {
        if references.len() < 2 {
            return;
        }

        let mut filtered = Vec::with_capacity(references.len());

        // keep a filtered list of non-overlapping references
        for reference in references.iter().copied() {
            let Some(last_reference) = filtered.last_mut() else {
                filtered.push(reference);
                continue;
            };
            let span = reference.span;
            let last_span = &mut last_reference.span;

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
}

/// Request find references at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FindReferencesRequest {
    /// The queried position.
    pub position: Position,
    /// Whether to include the declaration in results.
    pub include_declaration: bool,
}

/// Response payload for find references queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FindReferencesResponse {
    /// Reference occurrences.
    pub references: Vec<Reference>,
}

impl ModuleQueryContext<'_> {
    /// Find all references to the symbol at the given position.
    ///
    /// Optionally includes the declaration in the results.
    pub fn find_references(
        &self,
        program: &ProgramQueryContext<'_>,
        offset: u32,
        include_declaration: bool,
    ) -> Vec<Reference> {
        // find the symbol at offset
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };

        // preserve local import aliases as local reference targets
        let (target_symbol, declaration_span, target_name) =
            if let Some(local_alias_name) = self.local_import_alias_name(symbol_at.symbol_id) {
                let declaration_span = if include_declaration {
                    self.symbol_local_definition_span(symbol_at.symbol_id)
                } else {
                    None
                };

                (
                    symbol_at.symbol_id,
                    declaration_span,
                    Some(local_alias_name),
                )
            } else {
                let canonical_id = self.canonical_symbol(symbol_at.symbol_id);
                let canonical_name = self.symbol_name(canonical_id);
                let declaration_span = if include_declaration {
                    self.module_context(canonical_id.module_id)
                        .symbol_definition_span(canonical_id)
                } else {
                    None
                };

                (canonical_id, declaration_span, canonical_name)
            };

        // search all modules for references to that symbol
        let occurrences = self.find_references_to_symbol(
            program,
            target_symbol,
            declaration_span,
            target_name.as_deref(),
        );
        occurrences
            .into_iter()
            .map(|occurrence| Reference {
                target: Target::new(occurrence.module, occurrence.span)
                    .with_symbol_id(target_symbol),
                role: occurrence.role,
            })
            .collect()
    }

    /// Find all references to a symbol across all modules.
    fn find_references_to_symbol(
        &self,
        program: &ProgramQueryContext<'_>,
        canonical_id: dir::GlobalSymbolId,
        declaration_span: Option<Span>,
        target_name: Option<&str>,
    ) -> Vec<ReferenceOccurrence> {
        // initialize the reference list
        let mut references = Vec::new();

        // configure reference collection for find references behavior
        let reference_search = ReferenceFilter {
            include_expressions: true,
            include_members: true,
            include_dependency_items: true,
            include_namespace_receivers: true,
            skip_dependency_aliases: false,
            target_name,
            requires_target_name_match: false,
            limit_file: None,
        };

        let reference_spans =
            self.program_symbol_references(program, canonical_id, reference_search);
        references.extend(
            reference_spans
                .into_iter()
                .map(|(module, span)| ReferenceOccurrence::new(module, span, ReferenceRole::Read)),
        );

        // normalize ordering and remove duplicates
        references.sort_by_key(|reference| {
            (
                reference.span.file,
                reference.span.start,
                reference.span.end,
            )
        });
        references.dedup();
        ReferenceOccurrence::prune_overlaps(&mut references);
        self.sort_reference_spans(&mut references);

        // place the declaration first when requested
        if let Some(decl_span) = declaration_span {
            references.retain(|reference| {
                !(reference.span.file == decl_span.file
                    && reference.span.start == decl_span.start
                    && reference.span.end == decl_span.end)
            });
            references.insert(
                0,
                ReferenceOccurrence::new(self.module(), decl_span, ReferenceRole::Declaration),
            );
        }

        // return the final reference list
        references
    }

    /// Sort reference spans by stable file location.
    fn sort_reference_spans(&self, references: &mut [ReferenceOccurrence]) {
        references.sort_by(|left, right| {
            self.reference_span_order(left.span)
                .cmp(&self.reference_span_order(right.span))
        });
    }

    /// Return the stable order for a reference span.
    fn reference_span_order(&self, span: Span) -> ReferenceSpanOrder {
        let file = self.read_file(span.file);

        // prefer the displayed file name used by query snapshots
        let file_name = if !file.name.is_empty() {
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

        ReferenceSpanOrder::new(file_name, span)
    }
}

/// Stable source-location ordering for reference results.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceSpanOrder {
    /// The display file name.
    file: String,
    /// The span start offset.
    start: u32,
    /// The span end offset.
    end: u32,
    /// The stable file id.
    id: u64,
}

impl ReferenceSpanOrder {
    /// Build the ordering for one reference span.
    fn new(file: String, span: Span) -> Self {
        Self {
            file,
            start: span.start,
            end: span.end,
            id: span.file.0,
        }
    }
}
