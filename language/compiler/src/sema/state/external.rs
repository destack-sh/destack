use std::sync::Arc;

use elsa::FrozenMap;
use smallvec::SmallVec;
use tspp_artifact::{ArtifactProjectionKey, DirResolved, DirView, ModuleGraph};
use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_source::ModuleId;

use super::CheckState;
use crate::{CompilerError, CompilerResult};

/// External modules keyed by module id, read on first use through shared references.
pub(in crate::sema) struct ExternalModuleTable {
    /// One read external module per id.
    slots: FrozenMap<ModuleId, Box<DirView>>,
}

impl ExternalModuleTable {
    /// Return one external module already read by this pass.
    pub(in crate::sema) fn read(&self, module: ModuleId) -> Option<&DirView> {
        self.slots.get(&module)
    }
}

impl Default for ExternalModuleTable {
    fn default() -> Self {
        Self {
            slots: FrozenMap::new(),
        }
    }
}

impl<'a> CheckState<'a> {
    /// Return one external module's stages at this pass.
    pub(in crate::sema) fn external(
        &self,
        module: ModuleId,
    ) -> CompilerResult<Option<&'a DirView>> {
        if self.is_declaring() || self.is_own_module(module) {
            return Ok(None);
        }
        if let Some(view) = self.external_modules.slots.get(&module) {
            return Ok(Some(view));
        }

        // read the stages this pass reaches, a blocked stage yielding to the engine
        let view = self
            .pass
            .read_stages(self.artifacts, (module, self.profile))?;

        Ok(Some(
            self.external_modules.slots.insert(module, Box::new(view)),
        ))
    }

    /// Read one external module's resolved import targets.
    pub(in crate::sema) fn external_resolutions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Arc<DirResolved>> {
        if let Some(external) = self.external_modules.slots.get(&module)
            && let Some(resolved) = &external.resolved
        {
            return Ok(Arc::clone(resolved));
        }
        if let Some(resolved) = self.external_resolutions.get(&module) {
            return Ok(Arc::clone(resolved));
        }

        // read the resolve stage directly
        let resolved = self
            .artifacts
            .read::<DirResolved>((module, self.profile))
            .map_err(CompilerError::from)?;
        self.external_resolutions
            .insert(module, Arc::clone(&resolved));

        Ok(resolved)
    }

    /// Return the implementations of one interface declared across the program with their roots.
    pub(in crate::sema) fn program_implementations(
        &mut self,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::GlobalSymbolId, Option<dir::GlobalSymbolId>); 4]>> {
        // skip program reads while declaring
        if self.is_declaring() {
            return Ok(SmallVec::new());
        }

        // read the remembered implementations
        if let Some(symbols) = self.program_implementations.get(&interface) {
            return Ok(symbols.clone());
        }

        // select implementations from the graph of every package this module sees
        let packages = self
            .artifacts
            .package_closure(self.module_id.package_id)
            .map_err(CompilerError::from)?;
        let mut symbols = SmallVec::new();
        for package in packages {
            let selected = self
                .artifacts
                .project::<ModuleGraph, _, _>((package, self.profile), |graph| {
                    let symbols = graph
                        .interface_implementations(interface)
                        .iter()
                        .map(|implementation| (implementation.symbol, implementation.root))
                        .collect::<SmallVec<[_; 4]>>();
                    let projection = ArtifactProjectionKey::ModuleGraphImplementations(interface);

                    (symbols, [projection])
                })
                .map_err(CompilerError::from)?;
            for symbol in selected {
                if !symbols.contains(&symbol) {
                    symbols.push(symbol);
                }
            }
        }

        // remember the implementations for later queries
        self.program_implementations
            .insert(interface, symbols.clone());

        Ok(symbols)
    }

    /// Settle one symbol through import alias chains.
    pub(in crate::sema) fn resolve_symbol_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let mut current = symbol;
        let mut visited = FxIndexSet::default();

        // hop alias targets until a declaring symbol appears
        loop {
            if !visited.insert(current) {
                return Err(CompilerError::Internal {
                    message: format!("symbol alias {symbol:?} forwards in a cycle"),
                });
            }

            let resolved = if self.is_own_module(current.module_id) {
                Arc::clone(&self.module(current.module_id).resolved)
            } else {
                self.external_resolutions(current.module_id)?
            };
            let resolution = resolved.imports.symbol_resolution(current.local_id);
            match resolution {
                Some(dir::ImportResolution::Resolved(resolution))
                    if let dir::ExportTarget::Symbols(symbols) = &resolution.target
                        && let [target] = symbols.as_slice() =>
                {
                    current = *target;
                }
                Some(resolution) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "symbol alias {current:?} has no exact symbol target: {resolution:?}"
                        ),
                    });
                }
                None => {
                    // resolve import binders without a per-symbol target by their key
                    if let Some(target) = self.import_binder_target(current)?
                        && target != current
                    {
                        current = target;
                        continue;
                    }

                    return Ok(current);
                }
            }
        }
    }

    /// Return one import binder's resolved target symbol, when one exists.
    pub(in crate::sema) fn import_binder_target(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // read the binder's kind and key before releasing the table
        let table = self.binding_table(symbol.module_id)?;
        let binding = table.get_symbol(symbol.local_id);
        let kind = binding.kind;
        let key = binding.key;
        drop(table);

        // require an import symbol
        if kind != dir::SymbolKind::Import {
            return Ok(None);
        }

        // follow the module's own import resolutions first
        let resolved = if self.is_own_module(symbol.module_id) {
            Arc::clone(&self.module(symbol.module_id).resolved)
        } else {
            self.external_resolutions(symbol.module_id)?
        };
        if let Some(dir::ImportResolution::Resolved(resolution)) =
            resolved.imports.symbol_resolution(symbol.local_id)
            && let dir::ExportTarget::Symbols(symbols) = &resolution.target
            && let [target] = symbols.as_slice()
        {
            return Ok(Some(*target));
        }

        // follow resolved global names by the binder's key
        if let Some(key) = key
            && let Some([resolution]) = resolved
                .imports
                .global_resolution_by_key
                .get(&key)
                .map(Vec::as_slice)
            && let dir::ExportTarget::Symbols(symbols) = &resolution.target
            && let [target] = symbols.as_slice()
        {
            return Ok(Some(*target));
        }

        Ok(None)
    }

    /// Record which modules the checked module's direct imports make visible.
    pub(in crate::sema) fn import_external_modules(&mut self) -> CompilerResult<()> {
        // store the direct imports' visibility in every pass
        let module = self.module_id;
        let visible = self.external_module_ids(module);
        self.module_mut(module)
            .external_modules
            .extend(visible.iter().copied());

        Ok(())
    }

    /// Return external modules that can be named from the checked module.
    fn external_module_ids(&self, module: ModuleId) -> FxIndexSet<ModuleId> {
        let mut external_modules = FxIndexSet::default();
        let imports = &self.module(module).resolved.imports;

        // collect the modules the imports target and the modules their resolved names live in
        let targets = imports.target_modules();
        let homes = imports.symbol_targets().map(|(_, symbol)| symbol.module_id);
        for external_module in targets.chain(homes) {
            if !self.is_own_module(external_module) {
                external_modules.insert(external_module);
            }
        }

        external_modules
    }
}
