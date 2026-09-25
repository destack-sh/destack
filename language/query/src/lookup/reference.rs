use tspp_dir as dir;
use tspp_source::Span;

use crate::{Module, ModuleQueryContext, ProgramQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Iterate writable place targets selected during checking.
    pub(crate) fn writable_places(
        &self,
    ) -> QueryResult<impl Iterator<Item = (dir::GlobalNodeIdAny, &dir::WriteResolution)>> {
        let places = self
            .decisions()?
            .decision_entries()
            .filter_map(|(_, resolution)| match resolution {
                dir::Decision::Assignment(resolution) => {
                    Some((resolution.target, &resolution.write))
                }
                _ => None,
            });

        Ok(places)
    }
}

impl ProgramQueryContext<'_> {
    /// Return indexed program references to one symbol.
    pub(crate) fn symbol_references(
        &self,
        target: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<(Module, Span)>> {
        let mut references = Vec::new();

        // read only authored occurrences recorded in the program index
        for reference in self.symbol_program_references(target)? {
            let entry = reference.entry;
            references.push((reference.module, entry.span));
        }

        // remove exact duplicate index entries
        references.sort_by_key(|(module, span)| {
            (
                module.profile_id,
                module.module_id,
                span.file,
                span.start,
                span.end,
            )
        });
        references.dedup();

        Ok(references)
    }

    /// Return indexed references to one lexical declaration.
    pub(crate) fn declaration_references(
        &self,
        declaration: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<(Module, Span)>> {
        let mut references = Vec::new();

        // read only authored occurrences recorded for the declaration
        for reference in self.symbol_declaration_references(declaration)? {
            let entry = reference.entry;
            references.push((reference.module, entry.span));
        }

        // remove exact duplicate index entries
        references.sort_by_key(|(module, span)| {
            (
                module.profile_id,
                module.module_id,
                span.file,
                span.start,
                span.end,
            )
        });
        references.dedup();

        Ok(references)
    }
}
