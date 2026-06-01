use std::sync::Arc;

use destack_artifact::DirExported;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::resolve::state::{ExportLookupKey, ExportLookupState, ResolveState};
use crate::{CompilerError, CompilerResult};

/// Result of looking up an exported symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resolve) enum ExportLookup {
    /// A single export target was resolved.
    Found(dir::GlobalSymbolId),
    /// No matching export exists.
    Missing,
    /// Multiple star exports provide the same key.
    Ambiguous,
}

impl ResolveState<'_> {
    /// Resolve one exported symbol through direct and indirect exports.
    pub(in crate::resolve) fn resolve_export_symbol(
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

        let lookup = self.resolve_export_symbol_uncached(module, key)?;
        self.export_lookups
            .insert(cache_key, ExportLookupState::Resolved(lookup));

        Ok(lookup)
    }

    /// Return the cached export lookup when it is already known.
    fn cached_export_lookup(&self, key: ExportLookupKey) -> Option<ExportLookup> {
        match self.export_lookups.get(&key).copied() {
            Some(ExportLookupState::Resolved(lookup)) => Some(lookup),

            // break export cycles without hiding other star branches
            Some(ExportLookupState::Resolving) => Some(ExportLookup::Missing),

            None => None,
        }
    }

    /// Resolve one exported symbol without consulting the lookup cache.
    fn resolve_export_symbol_uncached(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let exported = self.exported_module(module)?;

        if let Some(export) = exported.exports.export_by_key.get(&key).copied() {
            return self.resolve_export_entry(module, export);
        }

        self.resolve_star_export_symbol(key, &exported.exports)
    }

    /// Return one exported module loaded through this provider run.
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
    fn resolve_export_entry(
        &mut self,
        module: ModuleId,
        export: dir::ExportEntry,
    ) -> CompilerResult<ExportLookup> {
        match export {
            dir::ExportEntry::Local(export) => {
                Ok(ExportLookup::Found(export.source.into_global(module)))
            }

            dir::ExportEntry::Indirect(export) => {
                let Some(target) = export.target else {
                    return Ok(ExportLookup::Missing);
                };
                let Some(key) = export.imported.selected_export_key() else {
                    return Ok(ExportLookup::Missing);
                };

                self.resolve_export_symbol(target, key)
            }
        }
    }

    /// Resolve one named export through star exports.
    fn resolve_star_export_symbol(
        &mut self,
        key: dir::ExportKey,
        exports: &dir::ExportTable,
    ) -> CompilerResult<ExportLookup> {
        if key == dir::ExportKey::Default {
            return Ok(ExportLookup::Missing);
        }

        let mut resolved = None;
        for export in exports.star_exports() {
            let Some(target) = export.target else {
                continue;
            };

            let symbol = match self.resolve_export_symbol(target, key)? {
                ExportLookup::Found(symbol) => symbol,
                ExportLookup::Missing => continue,
                ExportLookup::Ambiguous => return Ok(ExportLookup::Ambiguous),
            };

            if resolved.is_some_and(|resolved| resolved != symbol) {
                return Ok(ExportLookup::Ambiguous);
            }
            resolved = Some(symbol);
        }

        match resolved {
            Some(symbol) => Ok(ExportLookup::Found(symbol)),
            None => Ok(ExportLookup::Missing),
        }
    }
}
