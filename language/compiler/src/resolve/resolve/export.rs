use std::sync::Arc;

use destack_artifact::DirExported;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::resolve::state::{ExportLookupKey, ExportLookupState, ResolveState};
use crate::{CompilerError, CompilerResult};

/// Result of looking up an exported target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resolve) enum ExportLookup {
    /// One export target was resolved.
    Found(ExportTarget),
    /// Multiple star exports provide the same key.
    Ambiguous(Vec<ExportTarget>),
    /// No matching export exists.
    Missing,
}

/// One target resolved through an export surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resolve) enum ExportTarget {
    /// A symbol export target.
    Symbol(dir::GlobalSymbolId),
    /// A namespace export target.
    Namespace(ModuleId),
}

impl ExportTarget {
    /// Return this target as a path table target.
    ///
    /// Example:
    /// ```ds
    /// dep.api.value
    /// // dep.api can resolve to a namespace, dep.api.value can resolve to a symbol
    /// ```
    pub(in crate::resolve) fn path_target(self) -> dir::PathTarget {
        match self {
            Self::Symbol(symbol) => dir::PathTarget::Symbol(symbol),
            Self::Namespace(module) => dir::PathTarget::Namespace(module),
        }
    }
}

impl ResolveState<'_> {
    /// Resolve one exported target through direct and indirect exports.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// // value can come from dep.ds directly or through export { value } from "./inner.ds"
    /// ```
    pub(in crate::resolve) fn resolve_export_target(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let cache_key = ExportLookupKey { module, key };
        let is_cycle = matches!(
            self.export_lookups.get(&cache_key),
            Some(ExportLookupState::Resolving)
        );
        if let Some(lookup) = self.cached_export_lookup(cache_key) {
            self.stats.export_cache_hits += 1;
            if is_cycle {
                self.stats.export_cycle_hits += 1;
            }

            return Ok(lookup);
        }

        self.stats.export_cache_misses += 1;
        self.export_lookups
            .insert(cache_key, ExportLookupState::Resolving);

        let lookup = self.resolve_export_target_uncached(module, key)?;
        self.export_lookups
            .insert(cache_key, ExportLookupState::Resolved(lookup.clone()));

        Ok(lookup)
    }

    /// Return the cached export lookup when it is already known.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// import { value as other } from "./dep.ds";
    /// // the second lookup can reuse the first export result
    /// ```
    fn cached_export_lookup(&self, key: ExportLookupKey) -> Option<ExportLookup> {
        match self.export_lookups.get(&key).cloned() {
            Some(ExportLookupState::Resolved(lookup)) => Some(lookup),

            // break export cycles without hiding other star branches
            Some(ExportLookupState::Resolving) => Some(ExportLookup::Missing),

            None => None,
        }
    }

    /// Resolve one exported target without consulting the lookup cache.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// // dep.ds is loaded and searched directly on the first lookup
    /// ```
    fn resolve_export_target_uncached(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let exported = self.exported_module(module)?;

        if let Some(export) = exported.exports.export_by_key.get(&key).copied() {
            return self.resolve_export_entry(module, export);
        }

        self.resolve_star_export_target(key, &exported.exports)
    }

    /// Return one exported module loaded through this provider run.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    /// export { value } from "./dep.ds";
    /// // both clauses read the same exported module artifact
    /// ```
    fn exported_module(&mut self, module: ModuleId) -> CompilerResult<Arc<DirExported>> {
        if let Some(exported) = self.exported_modules.get(&module) {
            return Ok(exported.clone());
        }

        self.stats.export_table_loads += 1;

        let exported = self
            .artifacts
            .dir_exported(module, self.profile)
            .map_err(CompilerError::from)?;
        self.exported_modules.insert(module, exported.clone());

        Ok(exported)
    }

    /// Resolve one concrete export entry.
    ///
    /// Example:
    /// ```ds
    /// export { value };
    /// export { inner as value } from "./dep.ds";
    /// export * as api from "./api.ds";
    /// ```
    fn resolve_export_entry(
        &mut self,
        module: ModuleId,
        export: dir::ExportEntry,
    ) -> CompilerResult<ExportLookup> {
        match export {
            dir::ExportEntry::Local(export) => Ok(ExportLookup::Found(ExportTarget::Symbol(
                export.source.into_global(module),
            ))),

            dir::ExportEntry::Indirect(export) => {
                let Some(target) = export.target else {
                    return Ok(ExportLookup::Missing);
                };
                if export.imported == dir::ExportSelector::Namespace {
                    return Ok(ExportLookup::Found(ExportTarget::Namespace(target)));
                }
                let Some(key) = export.imported.selected_export_key() else {
                    return Ok(ExportLookup::Missing);
                };

                self.resolve_export_target(target, key)
            }
        }
    }

    /// Resolve one named export through star exports.
    ///
    /// Example:
    /// ```ds
    /// export * from "./a.ds";
    /// export * from "./b.ds";
    /// // importing { value } checks every visible star export
    /// ```
    fn resolve_star_export_target(
        &mut self,
        key: dir::ExportKey,
        exports: &dir::ExportTable,
    ) -> CompilerResult<ExportLookup> {
        if key == dir::ExportKey::Default {
            return Ok(ExportLookup::Missing);
        }

        let mut targets = Vec::new();

        // collect visible star export targets in declaration order
        for export in exports.star_exports() {
            let Some(target) = export.target else {
                continue;
            };

            match self.resolve_export_target(target, key)? {
                ExportLookup::Found(target) => {
                    insert_export_target(&mut targets, target);
                }
                ExportLookup::Ambiguous(ambiguous) => {
                    for target in ambiguous {
                        insert_export_target(&mut targets, target);
                    }
                }
                ExportLookup::Missing => {}
            }
        }

        // decide the final lookup shape
        match targets.as_slice() {
            [target] => Ok(ExportLookup::Found(*target)),
            [] => Ok(ExportLookup::Missing),
            _ => Ok(ExportLookup::Ambiguous(targets)),
        }
    }
}

/// Insert one export target if it is not already present.
///
/// Example:
/// ```ds
/// export * from "./a.ds";
/// export * from "./b.ds";
/// // duplicate targets from both star exports count once
/// ```
fn insert_export_target(targets: &mut Vec<ExportTarget>, target: ExportTarget) {
    if !targets.contains(&target) {
        targets.push(target);
    }
}
