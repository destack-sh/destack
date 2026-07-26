use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};

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
    pub(super) fn build(
        module: &'context ModuleQueryContext<'query>,
    ) -> ProviderResult<dir::ExtensionIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect checked extension declarations
        indexer.collect_extensions()?;

        Ok(dir::ExtensionIndex::new(indexer.entries))
    }

    /// Collect extension index entries.
    fn collect_extensions(&mut self) -> ProviderResult<()> {
        // collect checked extension declarations
        for (extension_symbol, extension) in self.module.definitions().iter_extensions() {
            // resolve extension definition source
            let source = self
                .module
                .definitions()
                .definition_source(extension_symbol)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "extension has no definition source: {extension_symbol:?}"
                    ))
                })?;

            // keep only declarations owned by this module
            if source.module_id != self.module.module_id() {
                continue;
            }

            // resolve source span and checked target
            let Some(span) = self.module.view().get_span_by_id(source.local_id.id) else {
                continue;
            };
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

        Ok(())
    }
}
