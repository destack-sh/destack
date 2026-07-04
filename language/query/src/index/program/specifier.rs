use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for specifier postings in one program index.
pub(super) struct SpecifierIndexer;

impl SpecifierIndexer {
    /// Build or reuse specifier postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::SpecifierPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.specifiers {
            if let Some(previous) = program.previous {
                return previous.specifiers.clone();
            }
        }

        // collect current module specifier indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.specifiers)
            .collect::<Vec<_>>();

        dir::SpecifierPostings::build(&indexes)
    }
}
