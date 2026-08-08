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

            // retain one row for the complete exported target
            if let Some(resolution) = self.resolve_export(export)? {
                self.entries.push(dir::ExportEntry {
                    name,
                    declaration: resolution.declaration,
                    target: resolution.target,
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

    /// Return the exact resolution of one named export.
    fn resolve_export(
        &self,
        export: &dir::NamedExport,
    ) -> ProviderResult<Option<dir::ExportResolution>> {
        match &export.binding {
            // resolve local declarations directly
            dir::ExportBinding::Local { symbols } => {
                let target = dir::ExportTarget::symbols(
                    symbols
                        .iter()
                        .map(|symbol| symbol.into_global(self.module_id)),
                )
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "local export {:?} has no declarations",
                        export.key
                    ))
                })?;

                // retain an authored public alias separately
                let declaration = match export.declaration {
                    Some(declaration) => {
                        dir::ExportTarget::symbol(declaration.into_global(self.module_id))
                    }
                    None => target.clone(),
                };

                Ok(Some(dir::ExportResolution {
                    declaration,
                    target,
                }))
            }

            // resolve imported declarations and their targets separately
            dir::ExportBinding::Import { .. } | dir::ExportBinding::ReExport { .. } => {
                // require the dependency item and its target
                let item = export.item.ok_or_else(|| {
                    ProviderError::internal(format!(
                        "indirect export {:?} has no dependency item",
                        export.key
                    ))
                })?;
                let source = item.into_global_any(self.module_id);
                let target = self.resolved.references.get(source).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "indirect export {item:?} has no resolved target"
                    ))
                })?;

                // retain the authored public declaration
                let declaration = match export.declaration {
                    Some(declaration) => Some(dir::ExportTarget::symbol(
                        declaration.into_global(self.module_id),
                    )),
                    None => {
                        let declaration =
                            self.resolved
                                .references
                                .declaration(source)
                                .ok_or_else(|| {
                                    ProviderError::internal(format!(
                                        "indirect export {item:?} has no resolved declaration"
                                    ))
                                })?;

                        self.resolve_reference(declaration, item)?
                    }
                };

                // retain rows with both declaration and target
                let target = self.resolve_reference(target, item)?;
                let (Some(declaration), Some(target)) = (declaration, target) else {
                    return Ok(None);
                };

                Ok(Some(dir::ExportResolution {
                    declaration,
                    target,
                }))
            }
        }
    }

    /// Resolve one export reference into its target.
    fn resolve_reference(
        &self,
        reference: &dir::Reference,
        item: dir::LocalNodeId<dir::DependencyItem>,
    ) -> ProviderResult<Option<dir::ExportTarget>> {
        match reference {
            dir::Reference::Bound(symbols) => Ok(Some(dir::ExportTarget::Symbols(symbols.clone()))),
            dir::Reference::Namespace(module) => Ok(Some(dir::ExportTarget::Namespace(*module))),
            dir::Reference::Ambiguous(_) | dir::Reference::Missing => Ok(None),
            dir::Reference::Projected { .. } => Err(ProviderError::internal(format!(
                "export dependency item {item:?} has a projected target"
            ))
            .into()),
        }
    }
}
