use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::CheckState;
use crate::{CompilerError, CompilerResult};

use super::name::NameLookup;

/// Result of looking up an exported symbol during check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum ExportLookup {
    /// A single export target was resolved.
    Found(dir::GlobalSymbolId),
    /// No matching export exists.
    Missing,
    /// Multiple star exports provide the same key.
    Ambiguous(SmallVec<[dir::GlobalSymbolId; 4]>),
}

/// Cache key for one exported name in one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ExportLookupKey {
    /// The module that owns the export table.
    pub(in crate::check) module: ModuleId,
    /// The export key being looked up.
    pub(in crate::check) key: dir::ExportKey,
}

/// Memoized export lookup state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum ExportLookupState {
    /// The lookup is currently being computed.
    Resolving,
    /// The lookup has been computed.
    Resolved(ExportLookup),
}

impl CheckState<'_> {
    /// Look up one source path that starts from lexical name lookup.
    pub(in crate::check) fn lookup_path_symbol(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        space: dir::SymbolSpace,
    ) -> CompilerResult<ExportLookup> {
        let Some((name, tail)) = path.segments.split_first() else {
            return Ok(ExportLookup::Missing);
        };
        let symbol = match self.lookup_symbol_by_name(module, source, *name, space) {
            // exactly one root symbol
            NameLookup::Found(symbol) => symbol,
            // no root symbol
            NameLookup::Missing => return Ok(ExportLookup::Missing),
            // multiple root symbols
            NameLookup::Ambiguous(symbols) => {
                return Ok(ExportLookup::Ambiguous(symbols));
            }
        };

        // return the root symbol for single segment paths
        if tail.is_empty() {
            return Ok(ExportLookup::Found(symbol));
        }

        let Some(module) = self.namespace_target_module(module, symbol)? else {
            return Ok(ExportLookup::Missing);
        };

        self.lookup_export_path_tail(module, tail)
    }

    /// Look up one exported symbol through direct and indirect exports.
    pub(in crate::check) fn lookup_export_symbol(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let cache_key = ExportLookupKey { module, key };
        if let Some(lookup) = self.cached_export_lookup(cache_key) {
            return Ok(lookup);
        }

        self.exports.insert(cache_key, ExportLookupState::Resolving);

        let lookup = self.compute_export_symbol(module, key)?;

        // cache the lookup without consuming the returned value
        self.exports
            .insert(cache_key, ExportLookupState::Resolved(lookup.clone()));

        Ok(lookup)
    }

    /// Return the cached export lookup when it is already known.
    fn cached_export_lookup(&self, key: ExportLookupKey) -> Option<ExportLookup> {
        match self.exports.get(&key).cloned() {
            Some(ExportLookupState::Resolved(lookup)) => Some(lookup),

            // break export cycles without hiding other star branches
            Some(ExportLookupState::Resolving) => Some(ExportLookup::Missing),

            None => None,
        }
    }

    /// Look up the tail of one namespace export path.
    fn lookup_export_path_tail(
        &mut self,
        mut module: ModuleId,
        tail: &[dir::StringId],
    ) -> CompilerResult<ExportLookup> {
        let Some((last, parents)) = tail.split_last() else {
            return Ok(ExportLookup::Missing);
        };

        // walk through namespace segments
        for segment in parents {
            let key = dir::ExportKey::named(dir::StaticKey::Name(*segment));
            let symbol = match self.lookup_export_symbol(module, key)? {
                // exactly one namespace symbol
                ExportLookup::Found(symbol) => symbol,
                // no namespace symbol
                ExportLookup::Missing => return Ok(ExportLookup::Missing),
                // multiple namespace symbols
                ExportLookup::Ambiguous(symbols) => return Ok(ExportLookup::Ambiguous(symbols)),
            };
            let Some(next) = self.namespace_target_module(symbol.module_id, symbol)? else {
                return Ok(ExportLookup::Missing);
            };

            module = next;
        }

        let key = dir::ExportKey::named(dir::StaticKey::Name(*last));

        self.lookup_export_symbol(module, key)
    }

    /// Return the target module selected by one namespace symbol.
    fn namespace_target_module(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ModuleId>> {
        if symbol.module_id != module {
            return Ok(None);
        }

        let local = symbol.local_id;
        let target = self.module(module).resolved.imports.symbol_target(local);
        let target = match target {
            Some(dir::ImportTarget::Namespace(module)) => Some(module),
            Some(dir::ImportTarget::Symbol(_)) | None => None,
        };

        Ok(target)
    }

    /// Compute one exported symbol without consulting the lookup cache.
    fn compute_export_symbol(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let artifacts = self.compiler.artifact_reader(self.context);
        let exported = artifacts
            .dir_exported(module, self.profile)
            .map_err(CompilerError::from)?;

        if let Some(export) = exported.exports.export_by_key.get(&key).copied() {
            return self.lookup_export_entry(module, export);
        }

        self.lookup_star_export_symbol(key, &exported.exports)
    }

    /// Look up one concrete export entry.
    fn lookup_export_entry(
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

                self.lookup_export_symbol(target, key)
            }
        }
    }

    /// Look up one named export through star exports.
    fn lookup_star_export_symbol(
        &mut self,
        key: dir::ExportKey,
        exports: &dir::ExportTable,
    ) -> CompilerResult<ExportLookup> {
        if key == dir::ExportKey::Default {
            return Ok(ExportLookup::Missing);
        }

        let mut resolved = None;

        // find the first concrete star export target
        for export in exports.star_exports() {
            let Some(target) = export.target else {
                continue;
            };

            let symbol = match self.lookup_export_symbol(target, key)? {
                ExportLookup::Found(symbol) => symbol,
                ExportLookup::Missing => continue,
                ExportLookup::Ambiguous(symbols) => return Ok(ExportLookup::Ambiguous(symbols)),
            };

            if resolved.is_some_and(|resolved| resolved != symbol) {
                let mut symbols = SmallVec::new();
                if let Some(resolved) = resolved {
                    symbols.push(resolved);
                }
                symbols.push(symbol);

                return Ok(ExportLookup::Ambiguous(symbols));
            }
            resolved = Some(symbol);
        }

        match resolved {
            Some(symbol) => Ok(ExportLookup::Found(symbol)),
            None => Ok(ExportLookup::Missing),
        }
    }

    /// Require a symbol named by one source path.
    pub(in crate::check) fn require_path_symbol(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
        space: dir::SymbolSpace,
    ) -> Option<dir::GlobalSymbolId> {
        let lookup = self
            .lookup_path_symbol(module, source, path, space)
            .unwrap_or_else(|_| panic!("export lookup failed for checked module {module:?}"));

        match lookup {
            ExportLookup::Found(symbol) => Some(symbol),
            ExportLookup::Missing => {
                self.report_unresolved_reference(module, source, path);

                None
            }
            ExportLookup::Ambiguous(_) => {
                self.report_ambiguous_reference(module, source, path);

                None
            }
        }
    }
}
