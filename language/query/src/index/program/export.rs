use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for export postings in one program index.
pub(super) struct ExportIndexer;

impl ExportIndexer {
    /// Build or reuse export postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::ExportPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.exports {
            if let Some(previous) = program.previous {
                return previous.exports.clone();
            }
        }

        // collect current module export indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.exports)
            .collect::<Vec<_>>();

        dir::ExportPostings::build(&indexes)
    }
}
