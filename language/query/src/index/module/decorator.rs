use destack_dir as dir;

use super::context::ModuleIndexContext;

/// Builder for one decorator index from checked DIR.
pub(in crate::index) struct DecoratorIndexer<'context, 'index> {
    /// The indexed module context.
    module: &'context ModuleIndexContext<'index>,
    /// The collected index entries.
    entries: Vec<dir::DecoratorEntry>,
}

impl<'context, 'index> DecoratorIndexer<'context, 'index> {
    /// Build the decorator index.
    pub(in crate::index) fn build(
        module: &'context ModuleIndexContext<'index>,
    ) -> dir::DecoratorIndex {
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
        for (application_id, application) in self.module.decorators().iter_applications() {
            // resolve decorator display name
            let name = self.module.decorator_name(application.source.local_id);

            // emit decorator application row
            self.entries.push(dir::DecoratorEntry {
                name,
                application: application_id,
                decorator: application.source,
                expression: application.expression,
                owner: application.owner,
                target: application.resolution.target,
            });
        }
    }
}
