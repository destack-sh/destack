use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for decorator postings in one program index.
pub(super) struct DecoratorIndexer;

impl DecoratorIndexer {
    /// Build or reuse decorator postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::DecoratorPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.decorators {
            if let Some(previous) = program.previous {
                return previous.decorators.clone();
            }
        }

        // collect current module decorator indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.decorators)
            .collect::<Vec<_>>();

        dir::DecoratorPostings::build(&indexes)
    }
}
