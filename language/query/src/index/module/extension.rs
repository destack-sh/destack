use destack_dir as dir;

use crate::ModuleQueryContext;

/// Builder for one extension index from checked DIR.
pub(super) struct ExtensionIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected index entries.
    entries: Vec<dir::ExtensionEntry>,
}

impl<'context, 'query> ExtensionIndexer<'context, 'query> {
    /// Build the extension index.
    pub(super) fn build(module: &'context ModuleQueryContext<'query>) -> dir::ExtensionIndex {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked extension declarations
        indexer.collect_extensions();

        dir::ExtensionIndex::new(indexer.entries)
    }

    /// Collect extension index entries.
    fn collect_extensions(&mut self) {
        // collect checked extension declarations
        for (extension_symbol, extension) in self.module.definitions().iter_extensions() {
            // resolve extension definition source
            let source = self
                .module
                .definitions()
                .definition_source(extension_symbol)
                .unwrap_or_else(|| panic!("missing extension source for {extension_symbol:?}"));

            // keep only declarations owned by this module
            if source.module_id != self.module.module_id() {
                continue;
            }

            // resolve source span and checked target
            let span = self.module.get_span(self.module.view(), source.local_id);
            let root = extension.target.root();

            // emit extension declaration row
            self.entries.push(dir::ExtensionEntry {
                declaration: extension_symbol,
                source,
                file: span.file,
                span,
                root,
                ty: extension.target.r#type(),
                form: extension.form,
            });
        }
    }
}
