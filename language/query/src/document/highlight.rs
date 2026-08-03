use std::collections::BTreeMap;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, Span};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult};

/// A highlighted range in a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Highlight {
    /// The highlighted range.
    pub range: Span,
    /// The occurrence role.
    pub kind: HighlightKind,
}

/// The source role of one highlighted occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum HighlightKind {
    /// A declaration or reference without a value access role.
    Text,
    /// A value read.
    Read,
    /// A value declaration or write.
    Write,
}

impl HighlightKind {
    /// Combine roles recorded for one authored occurrence.
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Write, _) | (_, Self::Write) => Self::Write,
            (Self::Read, _) | (_, Self::Read) => Self::Read,
            (Self::Text, Self::Text) => Self::Text,
        }
    }
}

/// Request highlights at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HighlightRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for highlight queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HighlightResponse {
    /// Highlights.
    pub highlights: Vec<Highlight>,
}

impl ModuleQueryContext<'_> {
    /// Highlight all occurrences of the symbol at the given position in the module.
    ///
    /// Only highlights within the same file (for cross file, use find_references).
    pub fn highlight(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Vec<Highlight>> {
        let Some(occurrence) = self.declaration_at_offset(file_id, offset)? else {
            return Ok(Vec::new());
        };

        // preserve one explicit local import alias
        let is_local_declaration = match occurrence.symbol() {
            Some(symbol) => self.local_import_alias_name(symbol)?.is_some(),
            None => false,
        };
        let symbols = if is_local_declaration {
            occurrence.symbols
        } else {
            let mut symbols = Vec::new();
            for symbol in occurrence.symbols {
                symbols.extend(program.canonical_symbols(symbol)?);
            }
            symbols.sort();
            symbols.dedup();
            symbols
        };
        let modification_spans = self.modification_spans(file_id)?;
        let mut highlights = BTreeMap::new();

        // collect definitions and references for every exact declaration
        for symbol in symbols {
            if symbol.module_id == self.module_id()
                && let Some(span) = self.symbol_local_definition_span(symbol)?
            {
                let kind = self.declaration_highlight_kind(program, symbol)?;
                Self::insert_highlight(&mut highlights, span, kind);
            }

            let indexed = if is_local_declaration {
                program.declaration_references(symbol, Some(file_id))?
            } else {
                program.symbol_references(symbol, Some(file_id))?
            };
            let reference_kind = self.reference_highlight_kind(program, symbol)?;
            for (_, span) in indexed {
                let kind = if modification_spans.contains(&span) {
                    HighlightKind::Write
                } else {
                    reference_kind
                };
                Self::insert_highlight(&mut highlights, span, kind);
            }
        }

        let highlights = highlights
            .into_iter()
            .map(|((file, start, end), kind)| Highlight {
                range: Span::new(file, start, end),
                kind,
            })
            .collect();

        Ok(highlights)
    }

    /// Insert or merge one highlighted source occurrence.
    fn insert_highlight(
        highlights: &mut BTreeMap<(FileId, u32, u32), HighlightKind>,
        span: Span,
        kind: HighlightKind,
    ) {
        highlights
            .entry((span.file, span.start, span.end))
            .and_modify(|existing| *existing = existing.merge(kind))
            .or_insert(kind);
    }

    /// Return the highlight role for one symbol declaration.
    fn declaration_highlight_kind(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<HighlightKind> {
        let module = program.module(symbol_id.module_id)?;
        let symbol = module.bindings()?.get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return Ok(HighlightKind::Text);
        };

        let kind = match declaration.local_id.ty {
            dir::NodeType::Pattern | dir::NodeType::Parameter | dir::NodeType::DependencyItem => {
                HighlightKind::Write
            }
            dir::NodeType::GenericParameter
                if symbol.kind == dir::SymbolKind::GenericValueParameter =>
            {
                HighlightKind::Write
            }
            _ => HighlightKind::Text,
        };

        Ok(kind)
    }

    /// Return the highlight role for one symbol reference.
    fn reference_highlight_kind(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<HighlightKind> {
        let module = program.module(symbol_id.module_id)?;
        let symbol = module.bindings()?.get_symbol(symbol_id.local_id);

        let kind = match symbol.kind {
            dir::SymbolKind::Variable
            | dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::GenericValueParameter
            | dir::SymbolKind::Import => HighlightKind::Read,
            _ => HighlightKind::Text,
        };

        Ok(kind)
    }

    /// Return authored writable place spans in one file.
    fn modification_spans(&self, file_id: FileId) -> QueryResult<FxHashSet<Span>> {
        let mut spans = FxHashSet::default();

        // project every checked write through the authored view
        for (target, _write) in self.writable_places()? {
            if target.module_id != self.module_id() {
                return Err(QueryError::invalid(format!("highlight write: {target:?}")));
            }
            let span = self
                .node_selection_span(self.view()?, target.local_id)?
                .ok_or(QueryError::missing(format!("highlight span: {target:?}")))?;
            if span.file == file_id {
                spans.insert(span);
            }
        }

        Ok(spans)
    }
}
