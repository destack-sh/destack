use destack_dir as dir;

use super::context::ModuleIndexContext;

/// Builder for one member index from checked DIR.
pub(crate) struct MemberIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected member declarations.
    entries: Vec<dir::MemberEntry>,
}

impl<'context, 'index> MemberIndexer<'context, 'index> {
    /// Build the member index.
    pub(crate) fn build(module: &'context ModuleIndexContext<'index>) -> dir::MemberIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect symbol-backed definition members
        indexer.collect_members();

        // retain exact checked interface conformance and class override edges
        let definitions = module.definitions();
        let conformances =
            definitions
                .member_conformances()
                .map(|conformance| dir::MemberImplementation {
                    declaration: conformance.requirement,
                    implementation: conformance.member,
                });
        let overrides =
            definitions
                .member_overrides()
                .map(|(member, base)| dir::MemberImplementation {
                    declaration: base,
                    implementation: member,
                });
        let implementations = conformances.chain(overrides).collect();

        dir::MemberIndex::new(indexer.entries, implementations)
    }

    /// Collect symbol-backed definition members owned by this module.
    fn collect_members(&mut self) {
        for (declaring, definition) in self.module.definitions().iter_definitions() {
            for member in definition.members() {
                // keep only symbol-backed declarations owned by this module
                let source = member.source();
                let Some(symbol) = member.symbol() else {
                    continue;
                };
                if source.module_id != self.module.module_id() {
                    continue;
                }

                self.entries.push(dir::MemberEntry {
                    source,
                    declaring,
                    symbol,
                });
            }
        }
    }
}
