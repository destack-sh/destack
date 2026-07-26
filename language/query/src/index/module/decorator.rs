use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one decorator index from checked DIR.
pub(super) struct DecoratorIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::DecoratorEntry>,
}

impl<'context, 'query> DecoratorIndexer<'context, 'query> {
    /// Build the decorator index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::DecoratorIndex {
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
