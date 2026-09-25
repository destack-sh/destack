use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::{ContextError, ModuleContext, ProgramContext};

impl ModuleContext {
    /// Return every symbol selected for one candidate node.
    pub fn symbol_targets(&self, node: dir::LocalNodeIdAny) -> Vec<dir::GlobalSymbolId> {
        let node = node.into_global(self.module());

        // prefer selected member declarations
        if let Some(resolution) = self.decisions().member_decision(node) {
            let mut symbols = Vec::new();
            for access in resolution.arms() {
                access.target.collect_symbols(&mut symbols);
            }

            return symbols;
        }

        // prefer selected function values
        if let Some(resolution) = self.decisions().function_decision(node) {
            let symbols = resolution
                .arms()
                .iter()
                .filter_map(|value| value.target.symbol())
                .collect::<Vec<_>>();
            if !symbols.is_empty() {
                return symbols;
            }
        }

        // read the name resolution
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
                let Some(resolutions) = self.resolved().imports.global_resolutions(key) else {
                    return Ok(Vec::new());
                };

                resolutions
                    .iter()
                    .flat_map(|resolution| resolution.target.iter())
                    .collect()
            }
            dir::SymbolLookup::Found(symbol) => {
                vec![dir::ReferenceTarget::Symbol(
                    symbol.into_global(self.module()),
                )]
            }
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| dir::ReferenceTarget::Symbol(symbol.into_global(self.module())))
                .collect(),
        };
        targets = program.dependency_targets(&targets)?;

        // walk declaration members and imported module exports
        for segment in path.segments.iter().skip(1) {
            let key = dir::StaticKey::Name(*segment);
            targets = program.target_members(&targets, key)?;
        }

        let targets = program.dependency_targets(&targets)?;
        let symbols = targets
            .into_iter()
            .filter_map(|target| match target {
                dir::ReferenceTarget::Symbol(symbol) => Some(symbol),
                dir::ReferenceTarget::Namespace(_) => None,
            })
            .collect();

        Ok(symbols)
    }
}

impl ProgramContext {
    /// Return declaration targets selected by symbol bindings.
    pub fn symbol_targets(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> Result<Vec<dir::GlobalSymbolId>, ContextError> {
        let targets = symbols
            .iter()
            .copied()
            .map(dir::ReferenceTarget::Symbol)
            .collect::<Vec<_>>();
        let targets = self.dependency_targets(&targets)?;
        let symbols = targets
            .into_iter()
            .filter_map(|target| match target {
                dir::ReferenceTarget::Symbol(symbol) => Some(symbol),
                dir::ReferenceTarget::Namespace(_) => None,
            })
            .collect();

        Ok(symbols)
    }

    /// Return symbols or module namespaces selected by dependency bindings.
    fn dependency_targets(
        &self,
        targets: &[dir::ReferenceTarget],
    ) -> Result<Vec<dir::ReferenceTarget>, ContextError> {
        let mut selected = Vec::new();

        // replace local import declarations with their target declarations
        for target in targets {
            let dir::ReferenceTarget::Symbol(symbol) = target else {
                selected.push(*target);
                continue;
            };
            let module = self.module(symbol.module_id)?;
            match module.resolved().imports.symbol_resolution(symbol.local_id) {
                Some(resolution) => selected.extend(resolution.targets()),
                None => selected.push(*target),
            }
        }
        selected.sort_unstable();
        selected.dedup();

        Ok(selected)
    }

    /// Select one named member from declarations or module namespaces.
    fn target_members(
        &self,
        targets: &[dir::ReferenceTarget],
        key: dir::StaticKey,
    ) -> Result<Vec<dir::ReferenceTarget>, ContextError> {
        let mut members = Vec::new();

        // apply the same member segment to every current target
        for target in targets {
            match target {
                dir::ReferenceTarget::Symbol(symbol) => {
                    let module = self.module(symbol.module_id)?;
                    let lookup = module.bindings().lookup_key_member(symbol.local_id, key);
                    match lookup {
                        dir::SymbolLookup::Missing => {}
                        dir::SymbolLookup::Found(member) => members.push(
                            dir::ReferenceTarget::Symbol(member.into_global(symbol.module_id)),
                        ),
                        dir::SymbolLookup::Ambiguous(symbols) => {
                            members.extend(symbols.into_iter().map(|member| {
                                dir::ReferenceTarget::Symbol(member.into_global(symbol.module_id))
                            }));
                        }
                    }
                }
                dir::ReferenceTarget::Namespace(module) => {
                    let key = dir::ExportKey::Named(key);
                    members.extend(self.export_targets(*module, key, &mut Vec::new())?);
                }
            }
        }

        self.dependency_targets(&members)
    }

    /// Resolve one named export through local, indirect, and star exports.
    fn export_targets(
        &self,
        module: ModuleId,
        key: dir::ExportKey,
        active: &mut Vec<(ModuleId, dir::ExportKey)>,
    ) -> Result<Vec<dir::ReferenceTarget>, ContextError> {
        let relation = (module, key);

        // stop repeated star export relations
        if active.contains(&relation) {
            return Ok(Vec::new());
        }
        active.push(relation);

        // select a direct named export
        let context = self.module(module)?;
        if let Some(export) = context.exported().exports.export_by_key.get(&key) {
            let targets = self.named_export_targets(module, export)?;
            active.pop();

            return Ok(targets);
        }

        // exclude default names from star exports
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
        export: &dir::NamedExport,
    ) -> Result<Vec<dir::ReferenceTarget>, ContextError> {
        // read the exact target from resolved dependency items
        if let Some(item) = export.item {
            let source = item.into_global_any(module);
            let context = self.module(module)?;
            let reference = context
                .resolved()
                .references
                .get(source)
                .ok_or(ContextError::InvalidReference(source))?;
            let targets = match reference {
                dir::Reference::Bound(symbols) => symbols
                    .iter()
                    .copied()
                    .map(dir::ReferenceTarget::Symbol)
                    .collect(),
                dir::Reference::Namespace { module, .. } => {
                    vec![dir::ReferenceTarget::Namespace(*module)]
                }
                dir::Reference::Ambiguous(targets) => targets.clone().into_vec(),
                dir::Reference::TypeLiteral(_) | dir::Reference::Missing => Vec::new(),
                dir::Reference::Projected { .. } => {
                    return Err(ContextError::InvalidReference(source));
                }
            };

            return Ok(targets);
        }

        // read the local symbol group from declaration exports
        match &export.binding {
            dir::ExportBinding::Local { symbols } => Ok(symbols
                .iter()
                .map(|symbol| dir::ReferenceTarget::Symbol(symbol.into_global(module)))
                .collect()),
            dir::ExportBinding::Import { .. } | dir::ExportBinding::ReExport { .. } => {
                Err(ContextError::InvalidExport(module, export.key))
            }
        }
    }
}
