use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for extension postings in one program index.
pub(super) struct ExtensionIndexer;

impl ExtensionIndexer {
    /// Build or reuse extension postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::ExtensionPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.extensions {
            if let Some(previous) = program.previous {
                return previous.extensions.clone();
            }
        }

        // collect current module extension indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.extensions)
            .collect::<Vec<_>>();

        dir::ExtensionPostings::build(&indexes)
    }
}
