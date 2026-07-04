use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for member postings in one program index.
pub(super) struct MemberIndexer;

impl MemberIndexer {
    /// Build or reuse member postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::MemberPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.members {
            if let Some(previous) = program.previous {
                return previous.members.clone();
            }
        }

        // collect current module member indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.members)
            .collect::<Vec<_>>();

        dir::MemberPostings::build(&indexes)
    }
}
