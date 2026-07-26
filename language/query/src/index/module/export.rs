use destack_dir as dir;
use destack_repository::ProviderResult;

use crate::ModuleQueryContext;

/// Builder for one export index from checked DIR.
pub(super) struct ExportIndexer<'context, 'query> {
    /// The indexed module context.
    module: &'context ModuleQueryContext<'query>,
    /// The collected exports.
    entries: Vec<dir::ExportEntry>,
}

impl<'context, 'query> ExportIndexer<'context, 'query> {
    /// Build the export index.
    pub(super) fn build(
        module: &'context ModuleQueryContext<'query>,
    ) -> ProviderResult<dir::ExportIndex> {
        let mut indexer = Self {
            module,
            entries: Vec::new(),
        };

        // collect the exact resolved exports
        indexer.collect_exports()?;

        Ok(dir::ExportIndex::new(indexer.entries))
    }

    /// Collect named export entries.
    fn collect_exports(&mut self) -> ProviderResult<()> {
        // collect this module's named export rows
        for (key, export) in self.module.exports().exports() {
            let Some(name) = self.export_name(*key) else {
                continue;
            };

            // retain one row per exact overload or namespace target
            for target in self.export_targets(*export)? {
                self.entries.push(dir::ExportEntry {
                    name: name.clone(),
                    target,
                });
            }
        }

        Ok(())
    }

    /// Return the source name for one export key.
    fn export_name(&self, key: dir::ExportKey) -> Option<String> {
        match key {
            dir::ExportKey::Default => Some("default".to_string()),
            dir::ExportKey::Named(dir::StaticKey::Name(name)) => {
                Some(self.module.strings().get(name).to_string())
            }
            dir::ExportKey::Named(dir::StaticKey::Index(index)) => Some(index.to_string()),
            dir::ExportKey::Named(dir::StaticKey::Symbol(_)) => None,
        }
    }

    /// Return every exact target for one named export.
    fn export_targets(&self, export: dir::NamedExport) -> ProviderResult<Vec<dir::ExportTarget>> {
        match export {
            dir::NamedExport::Local(export) => {
                let symbol = export.source.into_global(self.module.module_id());

                Ok(vec![dir::ExportTarget::Symbol(symbol)])
            }
            dir::NamedExport::Indirect(export) => {
                let targets = self.module.dependency_targets(export.item)?;
                let targets = targets
                    .into_iter()
                    .map(|target| match target {
                        dir::ImportTarget::Symbol(symbol) => dir::ExportTarget::Symbol(symbol),
                        dir::ImportTarget::Namespace(module) => {
                            dir::ExportTarget::Namespace(module)
                        }
                    })
                    .collect();

                Ok(targets)
            }
        }
    }
}
