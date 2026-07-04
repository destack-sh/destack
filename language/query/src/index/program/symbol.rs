use destack_dir as dir;

use super::ProgramIndexer;

/// Builder for symbol postings in one program index.
pub(super) struct SymbolIndexer;

impl SymbolIndexer {
    /// Build or reuse symbol postings.
    pub(super) fn build(program: &ProgramIndexer<'_>) -> dir::SymbolPostings {
        // reuse unchanged postings from the previous program index
        if !program.changes.symbols {
            if let Some(previous) = program.previous {
                return previous.symbols.clone();
            }
        }

        // collect current module symbol indexes
        let indexes = program
            .modules
            .iter()
            .map(|module| &module.index.symbols)
            .collect::<Vec<_>>();

        dir::SymbolPostings::build(&indexes)
    }
}
