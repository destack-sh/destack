use destack_dir as dir;
use destack_source::ModuleId;

use crate::{ContextError, ModuleContext, ProgramContext};

impl ModuleContext {
    /// Return every symbol selected by one checked candidate node.
    pub fn symbol_targets(&self, node: dir::LocalNodeIdAny) -> Vec<dir::GlobalSymbolId> {
        let node = node.into_global(self.module());

        // prefer selected member declarations
        if let Some(resolution) = self.decisions().member_decision(node) {
            let mut symbols = Vec::new();
            for access in resolution.iter() {
                access.target.collect_symbols(&mut symbols);
            }

            return symbols;
        }

        // prefer explicit generic instantiations
        if let Some(resolution) = self.decisions().instantiation_decision(node) {
            return vec![resolution.symbol];
        }

        // otherwise read the checked name resolution
        let Some(resolution) = self.resolutions().name_resolution(node) else {
            return Vec::new();
        };

        resolution.symbols().to_vec()
    }

    /// Resolve one predicate reference in the lexical scope of a candidate root.
    pub fn resolve_reference(
        &self,
        candidate: dir::LocalNodeIdAny,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
        program: &ProgramContext,
    ) -> Result<Vec<dir::GlobalSymbolId>, ContextError> {
        let Some(path) = tree.reference_path(expression) else {
            return Ok(Vec::new());
        };

        self.resolve_path(candidate, &path, program)
    }

    /// Resolve one predicate path in the lexical scope of a candidate root.
    pub fn resolve_path(
        &self,
        candidate: dir::LocalNodeIdAny,
        path: &dir::Path,
        program: &ProgramContext,
    ) -> Result<Vec<dir::GlobalSymbolId>, ContextError> {
        let Some(first) = path.segments.first().copied() else {
            return Ok(Vec::new());
        };
        let key = dir::StaticKey::Name(first);
        let lookup = self
            .bindings()
            .lookup_symbol_at(&self.view(), candidate, key);
        let mut targets = match lookup {
            dir::SymbolLookup::Missing => {
                let Some(targets) = self.resolved().imports.global_targets(key) else {
                    return Ok(Vec::new());
                };

                targets
                    .iter()
                    .copied()
                    .map(dir::ExportTarget::from)
                    .collect()
            }
            dir::SymbolLookup::Found(symbol) => {
                vec![dir::ExportTarget::Symbol(symbol.into_global(self.module()))]
            }
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| dir::ExportTarget::Symbol(symbol.into_global(self.module())))
                .collect(),
        };
        targets = program.canonical_targets(&targets)?;

        // walk declaration members and imported module exports
        for segment in path.segments.iter().skip(1) {
            let key = dir::StaticKey::Name(*segment);
            targets = program.target_members(&targets, key)?;
        }

        let targets = program.canonical_targets(&targets)?;
        let symbols = targets
            .into_iter()
            .filter_map(|target| match target {
                dir::ExportTarget::Symbol(symbol) => Some(symbol),
                dir::ExportTarget::Namespace(_) => None,
            })
            .collect();

        Ok(symbols)
    }
}

impl ProgramContext {
    /// Follow imports to canonical declaration symbols.
    pub fn canonical_symbols(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> Result<Vec<dir::GlobalSymbolId>, ContextError> {
        let targets = symbols
            .iter()
            .copied()
            .map(dir::ExportTarget::Symbol)
            .collect::<Vec<_>>();
        let targets = self.canonical_targets(&targets)?;
        let symbols = targets
            .into_iter()
            .filter_map(|target| match target {
                dir::ExportTarget::Symbol(symbol) => Some(symbol),
                dir::ExportTarget::Namespace(_) => None,
            })
            .collect();

        Ok(symbols)
    }

    /// Follow import bindings to canonical symbols or module namespaces.
    fn canonical_targets(
        &self,
        targets: &[dir::ExportTarget],
    ) -> Result<Vec<dir::ExportTarget>, ContextError> {
        let mut canonical = Vec::new();

        // resolve every overload or ambiguous target independently
        for target in targets {
            self.canonical_target(*target, &mut Vec::new(), &mut canonical)?;
        }
        canonical.sort_unstable();
        canonical.dedup();

        Ok(canonical)
    }

    /// Follow one import binding to its terminal target.
    fn canonical_target(
        &self,
        target: dir::ExportTarget,
        active: &mut Vec<dir::GlobalSymbolId>,
        canonical: &mut Vec<dir::ExportTarget>,
    ) -> Result<(), ContextError> {
        let dir::ExportTarget::Symbol(symbol) = target else {
            canonical.push(target);

            return Ok(());
        };
        if active.contains(&symbol) {
            return Err(ContextError::CyclicSymbol(symbol));
        }
        active.push(symbol);

        let module = self.module(symbol.module_id)?;
        let resolution = module.resolved().imports.symbol_resolution(symbol.local_id);
        match resolution {
            Some(dir::ImportResolution::Resolved(target)) => {
                self.canonical_target((*target).into(), active, canonical)?;
            }
            Some(dir::ImportResolution::Ambiguous(targets)) => {
                for target in targets {
                    self.canonical_target((*target).into(), active, canonical)?;
                }
            }
            Some(dir::ImportResolution::Missing) => {}
            None => canonical.push(dir::ExportTarget::Symbol(symbol)),
        }
        active.pop();

        Ok(())
    }

    /// Select one named member from declarations or module namespaces.
    fn target_members(
        &self,
        targets: &[dir::ExportTarget],
        key: dir::StaticKey,
    ) -> Result<Vec<dir::ExportTarget>, ContextError> {
        let mut members = Vec::new();

        // apply the same member segment to every current target
        for target in targets {
            match target {
                dir::ExportTarget::Symbol(symbol) => {
                    let module = self.module(symbol.module_id)?;
                    let lookup = module.bindings().lookup_key_member(symbol.local_id, key);
                    match lookup {
                        dir::SymbolLookup::Missing => {}
                        dir::SymbolLookup::Found(member) => members.push(
                            dir::ExportTarget::Symbol(member.into_global(symbol.module_id)),
                        ),
                        dir::SymbolLookup::Ambiguous(symbols) => {
                            members.extend(symbols.into_iter().map(|member| {
                                dir::ExportTarget::Symbol(member.into_global(symbol.module_id))
                            }));
                        }
                    }
                }
                dir::ExportTarget::Namespace(module) => {
                    let key = dir::ExportKey::Named(key);
                    members.extend(self.export_targets(*module, key, &mut Vec::new())?);
                }
            }
        }

        self.canonical_targets(&members)
    }

    /// Resolve one named export through local, indirect, and star exports.
    fn export_targets(
        &self,
        module: ModuleId,
        key: dir::ExportKey,
        active: &mut Vec<(ModuleId, dir::ExportKey)>,
    ) -> Result<Vec<dir::ExportTarget>, ContextError> {
        let relation = (module, key);
        if active.contains(&relation) {
            return Ok(Vec::new());
        }
        active.push(relation);

        let context = self.module(module)?;
        if let Some(export) = context.exported().exports.export_by_key.get(&key).copied() {
            let targets = self.named_export_targets(module, export, active)?;
            active.pop();

            return Ok(targets);
        }
        if key == dir::ExportKey::Default {
            active.pop();

            return Ok(Vec::new());
        }

        // merge the same named export across every visible star edge
        let mut targets = Vec::new();
        for export in context.exported().exports.star_exports() {
            let Some(module) = export.target else {
                continue;
            };
            targets.extend(self.export_targets(module, key, active)?);
        }
        active.pop();
        targets.sort_unstable();
        targets.dedup();

        Ok(targets)
    }

    /// Resolve one concrete local or indirect export.
    fn named_export_targets(
        &self,
        module: ModuleId,
        export: dir::NamedExport,
        active: &mut Vec<(ModuleId, dir::ExportKey)>,
    ) -> Result<Vec<dir::ExportTarget>, ContextError> {
        match export {
            dir::NamedExport::Local(export) => Ok(vec![dir::ExportTarget::Symbol(
                export.source.into_global(module),
            )]),
            dir::NamedExport::Indirect(export) => {
                let Some(module) = export.target else {
                    return Ok(Vec::new());
                };
                let Some(key) = export.imported.selected_export_key() else {
                    return Ok(vec![dir::ExportTarget::Namespace(module)]);
                };

                self.export_targets(module, key, active)
            }
        }
    }
}
