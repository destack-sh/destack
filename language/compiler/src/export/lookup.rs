use std::collections::HashMap;
use std::sync::Arc;

use tspp_artifact::DirExported;
use tspp_dir as dir;
use tspp_repository::ArtifactReader;
use tspp_source::{ModuleId, ProfileId};

use crate::{CompilerError, CompilerResult};

/// Result of looking up an exported target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExportLookup {
    /// One export declaration and final target were resolved.
    Found(dir::ExportResolution),
    /// Multiple star exports provide the same key.
    Ambiguous(Vec<dir::ExportResolution>),
    /// No matching export exists.
    Missing,
}

impl ExportLookup {
    /// Select one public declaration without changing final targets.
    pub(crate) fn with_declaration(self, declaration: dir::ExportTarget) -> Self {
        match self {
            Self::Found(resolution) => Self::Found(resolution.with_declaration(declaration)),
            Self::Ambiguous(resolutions) => Self::Ambiguous(
                resolutions
                    .into_iter()
                    .map(|resolution| resolution.with_declaration(declaration.clone()))
                    .collect(),
            ),
            Self::Missing => Self::Missing,
        }
    }

    /// Append unique resolutions to one collection.
    pub(crate) fn append(self, resolutions: &mut Vec<dir::ExportResolution>) {
        match self {
            Self::Found(resolution) => Self::append_resolution(resolutions, resolution),
            Self::Ambiguous(selected) => {
                for resolution in selected {
                    Self::append_resolution(resolutions, resolution);
                }
            }
            Self::Missing => {}
        }
    }

    /// Append one resolution unless its final target is already present.
    fn append_resolution(
        resolutions: &mut Vec<dir::ExportResolution>,
        resolution: dir::ExportResolution,
    ) {
        let is_present = resolutions
            .iter()
            .any(|existing| existing.target == resolution.target);
        if !is_present {
            resolutions.push(resolution);
        }
    }
}

/// Cache key for one exported name in one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ExportLookupKey {
    /// The module that owns the export table.
    module: ModuleId,
    /// The export key being looked up.
    key: dir::ExportKey,
}

/// Memoized export lookup state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExportLookupState {
    /// The lookup is currently resolving.
    Resolving,
    /// The lookup has resolved.
    Resolved(ExportLookup),
}

/// Memoized export lookups over one profile's exported modules.
#[derive(Debug)]
pub(crate) struct ExportResolver {
    /// The active profile.
    profile: ProfileId,
    /// Export lookups already resolved through this resolver.
    lookups: HashMap<ExportLookupKey, ExportLookupState>,
    /// Exported modules already loaded through this resolver.
    modules: HashMap<ModuleId, Arc<DirExported>>,
    /// Cache hits observed for stats.
    cache_hits: usize,
    /// Cache misses observed for stats.
    cache_misses: usize,
    /// Cycle hits observed for stats.
    cycle_hits: usize,
    /// Export table loads observed for stats.
    table_loads: usize,
}

impl ExportResolver {
    /// Create an empty export resolver for one profile.
    pub(crate) fn new(profile: ProfileId) -> Self {
        Self {
            profile,
            lookups: HashMap::new(),
            modules: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cycle_hits: 0,
            table_loads: 0,
        }
    }

    /// Resolve one exported target through direct and indirect exports.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
    /// // value can come from dep.tspp directly or through export { value } from "./inner.tspp"
    /// ```
    pub(crate) fn resolve_export_target(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let cache_key = ExportLookupKey { module, key };
        let is_cycle = matches!(
            self.lookups.get(&cache_key),
            Some(ExportLookupState::Resolving)
        );
        if let Some(lookup) = self.cached_export_lookup(cache_key) {
            self.cache_hits += 1;
            if is_cycle {
                self.cycle_hits += 1;
            }

            return Ok(lookup);
        }

        self.cache_misses += 1;
        self.lookups.insert(cache_key, ExportLookupState::Resolving);

        let lookup = self.resolve_export_target_uncached(artifacts, module, key)?;
        self.lookups
            .insert(cache_key, ExportLookupState::Resolved(lookup.clone()));

        Ok(lookup)
    }

    /// Return the cached export lookup when it is already known.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
    /// import { value as other } from "./dep.tspp";
    /// // the second lookup can reuse the first export result
    /// ```
    fn cached_export_lookup(&self, key: ExportLookupKey) -> Option<ExportLookup> {
        match self.lookups.get(&key).cloned() {
            Some(ExportLookupState::Resolved(lookup)) => Some(lookup),

            // break export cycles without hiding other star branches
            Some(ExportLookupState::Resolving) => Some(ExportLookup::Missing),

            None => None,
        }
    }

    /// Resolve one exported target without consulting the lookup cache.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
    /// // dep.tspp is loaded and searched directly on the first lookup
    /// ```
    fn resolve_export_target_uncached(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        let exported = self.exported_module(artifacts, module)?;

        if let Some(export) = exported.exports.export_by_key.get(&key) {
            return self.resolve_export_entry(artifacts, module, export);
        }

        self.resolve_star_export_target(artifacts, key, &exported.exports)
    }

    /// Return one exported module loaded through this provider run.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
    /// export { value } from "./dep.tspp";
    /// // both clauses read the same exported module artifact
    /// ```
    pub(crate) fn exported_module(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        module: ModuleId,
    ) -> CompilerResult<Arc<DirExported>> {
        if let Some(exported) = self.modules.get(&module) {
            return Ok(exported.clone());
        }

        self.table_loads += 1;

        let exported = artifacts
            .read::<DirExported>((module, self.profile))
            .map_err(CompilerError::from)?;
        self.modules.insert(module, exported.clone());

        Ok(exported)
    }

    /// Resolve one stored export binding.
    pub(crate) fn resolve_binding(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        module: ModuleId,
        binding: &dir::ExportBinding,
    ) -> CompilerResult<ExportLookup> {
        // resolve local exports directly
        let (target, selector) = match binding {
            dir::ExportBinding::Local { symbols } => {
                let target = dir::ExportTarget::symbols(
                    symbols.iter().map(|symbol| symbol.into_global(module)),
                )
                .ok_or_else(|| CompilerError::Internal {
                    message: "local export binding has no declarations".to_string(),
                })?;

                return Ok(ExportLookup::Found(dir::ExportResolution::direct(target)));
            }
            dir::ExportBinding::Import {
                module, selector, ..
            }
            | dir::ExportBinding::ReExport { module, selector } => (*module, *selector),
        };

        // unresolved module edges have no exported target
        let Some(target) = target else {
            return Ok(ExportLookup::Missing);
        };

        // namespace bindings target the module directly
        let Some(key) = selector.selected_export_key() else {
            let target = dir::ExportTarget::Namespace(target);

            return Ok(ExportLookup::Found(dir::ExportResolution::direct(target)));
        };

        // named and default bindings follow the target export table
        self.resolve_export_target(artifacts, target, key)
    }

    /// Resolve one profile-global entry.
    pub(crate) fn resolve_global_entry(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        module: ModuleId,
        entry: &dir::GlobalEntry,
    ) -> CompilerResult<ExportLookup> {
        let lookup = self.resolve_binding(artifacts, module, &entry.binding)?;

        // expose the authored declaration without replacing final targets
        let Some(declaration) = entry.declaration else {
            return Ok(lookup);
        };
        let declaration = dir::ExportTarget::symbol(declaration.into_global(module));

        Ok(lookup.with_declaration(declaration))
    }

    /// Resolve one concrete export entry.
    ///
    /// Example:
    /// ```tspp
    /// export { value };
    /// export { inner as value } from "./dep.tspp";
    /// export * as api from "./api.tspp";
    /// ```
    fn resolve_export_entry(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        module: ModuleId,
        export: &dir::NamedExport,
    ) -> CompilerResult<ExportLookup> {
        let lookup = self.resolve_binding(artifacts, module, &export.binding)?;

        // replace inherited declarations only when this export declares its own name
        match export.declaration {
            Some(declaration) => {
                let declaration = dir::ExportTarget::symbol(declaration.into_global(module));

                Ok(lookup.with_declaration(declaration))
            }
            None => Ok(lookup),
        }
    }

    /// Resolve one named export through star exports.
    ///
    /// Example:
    /// ```tspp
    /// export * from "./a.tspp";
    /// export * from "./b.tspp";
    /// // importing { value } checks every visible star export
    /// ```
    fn resolve_star_export_target(
        &mut self,
        artifacts: &ArtifactReader<'_>,
        key: dir::ExportKey,
        exports: &dir::ExportTable,
    ) -> CompilerResult<ExportLookup> {
        if key == dir::ExportKey::Default {
            return Ok(ExportLookup::Missing);
        }

        let mut resolutions = Vec::new();

        // collect visible star export targets in declaration order
        for export in exports.star_exports() {
            let Some(target) = export.target else {
                continue;
            };

            let lookup = self.resolve_export_target(artifacts, target, key)?;
            lookup.append(&mut resolutions);
        }

        // decide the final lookup shape
        match resolutions.as_slice() {
            [resolution] => Ok(ExportLookup::Found(resolution.clone())),
            [] => Ok(ExportLookup::Missing),
            _ => Ok(ExportLookup::Ambiguous(resolutions)),
        }
    }
}

impl ExportResolver {
    /// Return this resolver's observed lookup counters.
    pub(crate) fn stats(&self) -> ExportLookupStats {
        ExportLookupStats {
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            cycle_hits: self.cycle_hits,
            table_loads: self.table_loads,
        }
    }
}

/// Observed export lookup counters for phase stats.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExportLookupStats {
    /// Cache hits observed.
    pub(crate) cache_hits: usize,
    /// Cache misses observed.
    pub(crate) cache_misses: usize,
    /// Cycle hits observed.
    pub(crate) cycle_hits: usize,
    /// Export table loads observed.
    pub(crate) table_loads: usize,
}
