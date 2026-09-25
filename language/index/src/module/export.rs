use std::sync::Arc;

use tspp_artifact::{DirExported, DirResolved};
use tspp_core::{FxIndexMap, FxIndexSet, StringPool};
use tspp_dir as dir;
use tspp_repository::{ProviderError, ProviderResult};
use tspp_source::ModuleId;

/// Builder for one module export index.
pub(crate) struct ExportIndexer<'a> {
    /// The indexed module id.
    module_id: ModuleId,
    /// The exported declarations.
    exported: &'a DirExported,
    /// The resolved dependency targets.
    resolved: &'a DirResolved,
    /// The import closure's exported and resolved views for star exports.
    closure: &'a FxIndexMap<ModuleId, (Arc<DirExported>, Arc<DirResolved>)>,
    /// The shared string pool.
    strings: &'a StringPool,
    /// The collected exports.
    entries: Vec<dir::ExportEntry>,
}

impl<'a> ExportIndexer<'a> {
    /// Build the export index from resolved exports.
    pub(crate) fn build(
        module_id: ModuleId,
        exported: &'a DirExported,
        resolved: &'a DirResolved,
        closure: &'a FxIndexMap<ModuleId, (Arc<DirExported>, Arc<DirResolved>)>,
        strings: &'a StringPool,
    ) -> ProviderResult<dir::ExportIndex> {
        let mut indexer = Self {
            module_id,
            exported,
            resolved,
            closure,
            strings,
            entries: Vec::new(),
        };

        // collect the exact resolved exports, then the names the stars expose
        indexer.collect_exports()?;
        indexer.collect_star_exports()?;

        Ok(dir::ExportIndex::new(indexer.entries))
    }

    /// Collect named export entries.
    fn collect_exports(&mut self) -> ProviderResult<()> {
        // collect this module's named export entries
        for (key, export) in self.exported.exports.exports() {
            let Some(name) = self.export_name(*key) else {
                continue;
            };

            // retain one entry for the complete exported target
            if let Some(resolution) = self.resolve_export(self.module_id, self.resolved, export)? {
                self.entries.push(dir::ExportEntry {
                    name,
                    declaration: resolution.declaration,
                    target: resolution.target,
                });
            }
        }

        Ok(())
    }

    /// Collect the entries visible through this module's star exports.
    fn collect_star_exports(&mut self) -> ProviderResult<()> {
        // shadow the star export set with this module's own named exports
        let shadowed = self
            .exported
            .exports
            .export_by_key
            .keys()
            .copied()
            .collect();

        // walk the star edges, collecting each key's declaring modules
        let mut declared = FxIndexMap::<dir::ExportKey, Vec<ModuleId>>::default();
        let mut visited = vec![self.module_id];
        self.collect_star_keys(
            &self.exported.exports,
            &shadowed,
            &mut visited,
            &mut declared,
        );

        // index each key the stars expose at its declaring modules
        for (key, declarers) in declared {
            let Some(name) = self.export_name(key) else {
                continue;
            };
            for declarer in declarers {
                let Some((exported, resolved)) = self.closure.get(&declarer) else {
                    continue;
                };
                let Some(export) = exported.exports.export_by_key.get(&key).cloned() else {
                    continue;
                };
                if let Some(resolution) = self.resolve_export(declarer, resolved, &export)? {
                    self.entries.push(dir::ExportEntry {
                        name: name.clone(),
                        declaration: resolution.declaration,
                        target: resolution.target,
                    });
                }
            }
        }

        Ok(())
    }

    /// Collect the keys one export table exposes through its stars.
    fn collect_star_keys(
        &self,
        exports: &dir::ExportTable,
        shadowed: &FxIndexSet<dir::ExportKey>,
        visited: &mut Vec<ModuleId>,
        declared: &mut FxIndexMap<dir::ExportKey, Vec<ModuleId>>,
    ) {
        for star in exports.star_exports() {
            // break star cycles at their first revisit
            let Some(target) = star.target else {
                continue;
            };
            if visited.contains(&target) {
                continue;
            }
            visited.push(target);

            // look the target's own exports up in the import closure
            let Some((exported, _)) = self.closure.get(&target) else {
                continue;
            };

            // record the target's named keys that no nearer module shadows
            for (key, _) in exported.exports.exports() {
                if *key == dir::ExportKey::Default || shadowed.contains(key) {
                    continue;
                }
                let declarers = declared.entry(*key).or_default();
                if !declarers.contains(&target) {
                    declarers.push(target);
                }
            }

            // shadow the target's deeper stars with its own names
            let mut deeper = shadowed.clone();
            deeper.extend(exported.exports.export_by_key.keys().copied());
            self.collect_star_keys(&exported.exports, &deeper, visited, declared);
        }
    }

    /// Return the source name for one export key.
    fn export_name(&self, key: dir::ExportKey) -> Option<String> {
        match key {
            dir::ExportKey::Default => Some("default".to_string()),
            dir::ExportKey::Named(dir::StaticKey::Name(name)) => {
                Some(self.strings.get(name).to_string())
            }
            dir::ExportKey::Named(dir::StaticKey::Index(index)) => Some(index.to_string()),
        }
    }

    /// Return the exact resolution of one named export.
    fn resolve_export(
        &self,
        module_id: ModuleId,
        resolved: &DirResolved,
        export: &dir::NamedExport,
    ) -> ProviderResult<Option<dir::ExportResolution>> {
        match &export.binding {
            // resolve local declarations directly
            dir::ExportBinding::Local { symbols } => {
                let target = dir::ExportTarget::symbols(
                    symbols.iter().map(|symbol| symbol.into_global(module_id)),
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
                        dir::ExportTarget::symbol(declaration.into_global(module_id))
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
                let source = item.into_global_any(module_id);
                let target = resolved.references.get(source).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "indirect export {item:?} has no resolved target"
                    ))
                })?;

                // retain the authored public declaration
                let declaration = match export.declaration {
                    Some(declaration) => Some(dir::ExportTarget::symbol(
                        declaration.into_global(module_id),
                    )),
                    None => {
                        let declaration =
                            resolved.references.declaration(source).ok_or_else(|| {
                                ProviderError::internal(format!(
                                    "indirect export {item:?} has no resolved declaration"
                                ))
                            })?;

                        self.resolve_reference(declaration, item)?
                    }
                };

                // retain entries with both declaration and target
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
            dir::Reference::Namespace { module, .. } => {
                Ok(Some(dir::ExportTarget::Namespace(*module)))
            }
            dir::Reference::Ambiguous(_)
            | dir::Reference::TypeLiteral(_)
            | dir::Reference::Missing => Ok(None),
            dir::Reference::Projected { .. } => Err(ProviderError::internal(format!(
                "export dependency item {item:?} has a projected target"
            ))
            .into()),
        }
    }
}
