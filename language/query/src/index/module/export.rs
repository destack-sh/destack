use destack_artifact::{DirExported, DirResolved};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ProviderError, ProviderResult};
use destack_source::ModuleId;

/// Builder for one module export index.
pub(in crate::index) struct ExportIndexer<'a> {
    /// The indexed module id.
    module_id: ModuleId,
    /// The exported declarations.
    exported: &'a DirExported,
    /// The resolved dependency targets.
    resolved: &'a DirResolved,
    /// The shared string pool.
    strings: &'a StringPool,
    /// The collected exports.
    entries: Vec<dir::ExportEntry>,
}

impl<'a> ExportIndexer<'a> {
    /// Build the export index from resolved exports.
    pub(in crate::index) fn build(
        module_id: ModuleId,
        exported: &'a DirExported,
        resolved: &'a DirResolved,
        strings: &'a StringPool,
    ) -> ProviderResult<dir::ExportIndex> {
        let mut indexer = Self {
            module_id,
            exported,
            resolved,
            strings,
            entries: Vec::new(),
        };

        // collect the exact resolved exports
        indexer.collect_exports()?;

        Ok(dir::ExportIndex::new(indexer.entries))
    }

    /// Collect named export entries.
    fn collect_exports(&mut self) -> ProviderResult<()> {
        // collect this module's named export rows
        for (key, export) in self.exported.exports.exports() {
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
                Some(self.strings.get(name).to_string())
            }
            dir::ExportKey::Named(dir::StaticKey::Index(index)) => Some(index.to_string()),
            dir::ExportKey::Named(dir::StaticKey::Symbol(_)) => None,
        }
    }

    /// Return every exact target for one named export.
    fn export_targets(&self, export: dir::NamedExport) -> ProviderResult<Vec<dir::ExportTarget>> {
        match export {
            dir::NamedExport::Local(export) => {
                let symbol = export.source.into_global(self.module_id);

                Ok(vec![dir::ExportTarget::Symbol(symbol)])
            }
            dir::NamedExport::Indirect(export) => {
                let source = export.item.into_global_any(self.module_id);
                let reference = self.resolved.references.get(source).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "export dependency has no resolved reference: {source:?}"
                    ))
                })?;
                let targets = match reference {
                    dir::Reference::Bound(symbols) => symbols
                        .iter()
                        .copied()
                        .map(dir::ExportTarget::Symbol)
                        .collect(),
                    dir::Reference::Namespace(module) => {
                        vec![dir::ExportTarget::Namespace(*module)]
                    }
                    dir::Reference::Ambiguous(targets) => targets
                        .iter()
                        .map(|target| match target {
                            dir::ImportTarget::Symbol(symbol) => dir::ExportTarget::Symbol(*symbol),
                            dir::ImportTarget::Namespace(module) => {
                                dir::ExportTarget::Namespace(*module)
                            }
                        })
                        .collect(),
                    dir::Reference::Missing => Vec::new(),
                    dir::Reference::Projected { .. } => {
                        return Err(ProviderError::internal(format!(
                            "export dependency has a projected reference: {source:?}"
                        ))
                        .into());
                    }
                };

                Ok(targets)
            }
        }
    }
}
