use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for reference postings in one program index.
pub(super) struct ReferenceIndexer;

impl ReferenceIndexer {
    /// Build or reuse reference postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::ReferencePostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.references {
            if let Some(previous) = program.previous {
                return previous.references.clone();
            }
        }

        // collect current module reference indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.references)
            .collect::<Vec<_>>();

        dir::ReferencePostings::build(&indexes)
    }
}
