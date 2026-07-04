use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for call postings in one program index.
pub(super) struct CallIndexer;

impl CallIndexer {
    /// Build or reuse call postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::CallPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.calls {
            if let Some(previous) = program.previous {
                return previous.calls.clone();
            }
        }

        // collect current module call indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.calls)
            .collect::<Vec<_>>();

        dir::CallPostings::build(&indexes)
    }
}
