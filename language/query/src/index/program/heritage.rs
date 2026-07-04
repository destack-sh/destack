use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for heritage postings in one program index.
pub(super) struct HeritageIndexer;

impl HeritageIndexer {
    /// Build or reuse heritage postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::HeritagePostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.heritage {
            if let Some(previous) = program.previous {
                return previous.heritage.clone();
            }
        }

        // collect current module heritage indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.heritage)
            .collect::<Vec<_>>();

        dir::HeritagePostings::build(&indexes)
    }
}
