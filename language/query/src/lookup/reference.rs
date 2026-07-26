use destack_dir as dir;
use destack_source::{FileId, Span};

use crate::{Module, ModuleQueryContext, ProgramQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Iterate writable places selected during checking.
    pub(crate) fn writable_places(&self) -> impl Iterator<Item = &dir::PlaceResolution> {
        let places = self
            .resolutions()
            .place_entries()
            .map(|(_, resolution)| resolution);
        let assignments =
            self.resolutions()
                .assign_pattern_entries()
                .filter_map(|(_, resolution)| match resolution {
                    dir::AssignPatternResolution::Place(place) => Some(place),
                    dir::AssignPatternResolution::Default(_)
                    | dir::AssignPatternResolution::Sequence(_)
                    | dir::AssignPatternResolution::Tuple(_)
                    | dir::AssignPatternResolution::Object(_) => None,
                });

        places.chain(assignments)
    }
}

impl ProgramQueryContext<'_> {
    /// Return indexed program references to one symbol.
    pub(crate) fn symbol_references(
        &self,
        target: dir::GlobalSymbolId,
        file: Option<FileId>,
    ) -> QueryResult<Vec<(Module, Span)>> {
        let mut references = Vec::new();

        // read only authored occurrences recorded in the program index
        for reference in self.symbol_program_references(target)? {
            let entry = reference.entry;
            if file.is_some_and(|file| entry.span.file != file) {
                continue;
            }

            references.push((reference.module, entry.span));
        }

        // remove exact duplicate index rows
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
        file: Option<FileId>,
    ) -> QueryResult<Vec<(Module, Span)>> {
        let mut references = Vec::new();

        // read only authored occurrences recorded for the declaration
        for reference in self.symbol_declaration_references(declaration)? {
            let entry = reference.entry;
            if file.is_some_and(|file| entry.span.file != file) {
                continue;
            }

            references.push((reference.module, entry.span));
        }

        // remove exact duplicate index rows
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
