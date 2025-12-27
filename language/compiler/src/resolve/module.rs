use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_dir::{Declaration, DependencyItem, Expression, LocalSymbolId};

use destack_source::ModuleId;
use destack_workspace::{ImportMeta, ModuleDir, ProfileId};

impl Compiler {
    /// Prepare the per profile DIR by cloning from the base DIR.
    pub(super) fn resolve_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        if module
            .dirs
            .iter()
            .any(|dir| dir.profile_id == Some(profile_id))
        {
            return Ok(());
        }
        let base = module.dir_base();

        // profile
        let profile = self.program.profile(profile_id);

        // import meta
        let import_meta = ImportMeta {
            url: module.uri.clone(),
            file: module.path.clone(),
            dir: module
                .path
                .as_ref()
                .and_then(|path| path.parent().map(|parent| parent.to_path_buf())),
            platform: profile.key.platform,
            runtime: profile.key.runtime,
            debug: profile.key.debug,
            env: profile.key.env.clone(),
        };

        let mut dir = ModuleDir::from_base(base, profile_id);
        dir.import_meta = Some(import_meta);
        module.dirs.push(dir);
        Ok(())
    }

    /// Resolve expressions, dependencies, and declarations (phase 1).
    pub(super) fn resolve_module_direct(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let mut tree = dir.tree.write();
        let mut symbols = dir.symbols.write();
        let mut collector = TaskResultCollector::new();

        // resolve expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.collect(
                &mut collector,
                self.resolve_expression(
                    &module,
                    dir,
                    profile,
                    expression_id,
                    &mut tree,
                    &mut symbols,
                ),
            );
        }

        // resolve dependencies
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            self.collect(
                &mut collector,
                self.resolve_dependency_item(
                    &module,
                    dir,
                    profile,
                    item_id,
                    &mut tree,
                    &mut symbols,
                ),
            );
        }

        // resolve declarations (e.g., extension target_symbol)
        for declaration_id in tree.iter_node_ids_of_type::<Declaration>() {
            self.collect(
                &mut collector,
                self.resolve_declaration(&module, dir, declaration_id, &mut tree, &mut symbols),
            );
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        Ok(())
    }

    /// Compute canonical_symbol for all symbols (phase 2).
    pub(super) fn resolve_module_canonical(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // collect symbols that have target_symbol but no canonical_symbol
        // (only include symbols with primary_declaration, others are internal/incomplete)
        let symbols_to_resolve: Vec<_> = (0..symbols.symbol_count())
            .map(LocalSymbolId::new)
            .filter_map(|id| {
                let symbol = symbols.get_symbol(id);
                if symbol.target_symbol.is_some()
                    && symbol.canonical_symbol.is_none()
                    && symbol.primary_declaration.is_some()
                {
                    Some((
                        id.into_global(module_id),
                        symbol.primary_declaration.unwrap(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        // drop locks before resolving canonical symbols (may need to access other modules)
        drop(symbols);
        drop(tree);
        drop(module);

        // resolve canonical symbols (may yield for cross-module resolution)
        let mut collector = TaskResultCollector::new();
        for (symbol_id, node) in symbols_to_resolve {
            self.collect(
                &mut collector,
                self.resolve_canonical_symbol(node, symbol_id, profile),
            );
        }

        // yield on any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        Ok(())
    }
}
