use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::Span;

use crate::{
    Module, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult, Target,
};

/// One symbol reference occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ReferenceOccurrence {
    /// The reference source.
    pub target: Target,
    /// The referenced symbols.
    pub symbols: Vec<dir::GlobalSymbolId>,
}

/// A find references request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FindReferencesRequest {
    /// The queried position.
    pub position: QueryPosition,
    /// Whether to include the declaration in results.
    pub include_declaration: bool,
}

/// A find references response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FindReferencesResponse {
    /// Reference occurrences.
    pub references: Vec<ReferenceOccurrence>,
}

impl ModuleQueryContext<'_> {
    /// Find all references to the symbol at the given position.
    ///
    /// Optionally includes the declaration in the results.
    pub fn find_references(
        &self,
        request: FindReferencesRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<FindReferencesResponse> {
        // read the exact local alias or selected identities at the cursor
        let position = request.position;
        let Some(occurrence) = self
            .cursor(position.file_id, position.offset)?
            .reference(program)?
        else {
            return Ok(FindReferencesResponse {
                references: Vec::new(),
            });
        };
        let is_local_alias = match occurrence.symbol() {
            Some(symbol) => self.is_local_import_alias(symbol)?,
            None => false,
        };
        let symbols = if is_local_alias {
            occurrence.symbols
        } else {
            let mut symbols = Vec::new();
            for symbol in occurrence.symbols {
                symbols.extend(program.symbol_targets(symbol)?);
            }
            symbols.sort();
            symbols.dedup();
            symbols
        };
        if symbols.is_empty() {
            return Ok(FindReferencesResponse {
                references: Vec::new(),
            });
        }

        let references = self.find_symbol_references(
            program,
            &symbols,
            request.include_declaration,
            is_local_alias,
        )?;

        Ok(FindReferencesResponse { references })
    }

    /// Find all references to symbols across all modules.
    fn find_symbol_references(
        &self,
        program: &ProgramQueryContext<'_>,
        symbols: &[dir::GlobalSymbolId],
        include_declaration: bool,
        is_local_alias: bool,
    ) -> QueryResult<Vec<ReferenceOccurrence>> {
        let mut declarations = Vec::new();
        let mut references = Vec::new();

        // collect each exact declaration and indexed occurrence
        for symbol in symbols {
            if include_declaration {
                let module = program.module(symbol.module_id)?;
                let span = if is_local_alias {
                    module.symbol_local_definition_span(program, *symbol)?
                } else {
                    program.symbol_definition_span(*symbol)?
                };
                let span = span.ok_or(QueryError::missing(format!(
                    "reference declaration: {:?}",
                    *symbol
                )))?;
                declarations.push((*symbol, module.module(), span));
            }

            let indexed = if is_local_alias {
                program.declaration_references(*symbol)?
            } else {
                program.symbol_references(*symbol)?
            };
            references.extend(
                indexed
                    .into_iter()
                    .map(|(module, span)| (*symbol, module, span)),
            );
        }

        // place declarations before references without duplicating their source spans
        let declarations = Self::group_references(declarations);
        let mut references = Self::group_references(references);
        references.retain(|reference| {
            !declarations
                .iter()
                .any(|declaration| declaration.target == reference.target)
        });
        references.splice(0..0, declarations);

        Ok(references)
    }

    /// Group symbol identities recorded at the same source occurrence.
    fn group_references(
        entries: Vec<(dir::GlobalSymbolId, Module, Span)>,
    ) -> Vec<ReferenceOccurrence> {
        let mut entries = entries;
        entries.sort_by_key(|(symbol, module, span)| {
            (
                module.profile_id,
                module.module_id,
                span.file,
                span.start,
                span.end,
                *symbol,
            )
        });

        let mut references: Vec<ReferenceOccurrence> = Vec::new();
        for (symbol, module, span) in entries {
            let target = Target::new(module, span);
            if let Some(reference) = references
                .last_mut()
                .filter(|reference| reference.target == target)
            {
                reference.symbols.push(symbol);
            } else {
                references.push(ReferenceOccurrence {
                    target,
                    symbols: vec![symbol],
                });
            }
        }

        references
    }
}
