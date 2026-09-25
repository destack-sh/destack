use tspp_dir as dir;

use super::context::ModuleIndexContext;

/// Builder for one decorator index from checked DIR.
pub(crate) struct DecoratorIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected index entries.
    entries: Vec<dir::DecoratorEntry>,
}

impl<'context, 'index> DecoratorIndexer<'context, 'index> {
    /// Build the decorator index.
    pub(crate) fn build(module: &'context ModuleIndexContext<'index>) -> dir::DecoratorIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked decorator applications
        indexer.collect_decorators();

        dir::DecoratorIndex::new(indexer.entries)
    }

    /// Collect checked decorator applications.
    fn collect_decorators(&mut self) {
        for (_, application) in self.module.decorators().iter_applications() {
            // resolve decorator display name
            let name = self.module.decorator_name(application.source.local_id);

            // emit decorator application entry
            self.entries.push(dir::DecoratorEntry {
                name,
                decorator: application.source,
                owner: application.owner,
                target: application.resolution.target,
            });
        }
    }
}
