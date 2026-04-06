use crate::timing::tags;
use crate::{Compiler, CompilerContext, ResolveResult};
use destack_artifact::{
    ArtifactKey, ArtifactStamp, DirPrepared, ExportedSymbolTable, ImportedModuleTable,
    ModuleBindingExportTable, TargetEnv, TargetVendor,
};
use destack_dir::{DependencyItem, Export, GlobalSymbolId, LocalSymbolId};
use destack_source::ModuleId;
use destack_workspace::{ImportMeta, ImportMetaTarget, ProfileId};

impl Compiler {
    /// Prepare the per profile DIR by cloning from the base DIR.
    pub(crate) fn resolve_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        artifact_key: ArtifactKey,
        artifact_stamp: ArtifactStamp,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE);

        self.require_dir_base(revision, module_id)?;

        // reuse one persisted prepared dir image after the current file state is known
        if self
            .restore_cached_artifact(
                revision,
                artifact_key,
                |compiler| {
                    compiler.load_dir_prepared_image(
                        revision,
                        module_id,
                        artifact_stamp,
                        profile_id,
                    )
                },
                |store, version, payload| store.publish_dir_prepared(version, payload),
            )
            .is_some()
        {
            tracing::trace!(?module_id, ?profile_id, "resolve.module.prepare.cache_hit");
            return Ok(());
        }

        // data/text/binary modules have simpler preparation
        if !context.is_code_module(module_id) {
            return self.resolve_data_module_prepare(module_id, profile_id);
        }

        // load the module
        let module = context.module(module_id);
        let module = module.as_ref();
        // load the base dir and profile
        let base = self
            .artifact_dir_base(module_id)
            .unwrap_or_else(|| panic!("missing committed base dir artifact for {module_id:?}"));
        let profile = self.profile(profile_id);
        let path = module.path.clone();
        let dir = module
            .path
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));

        // build import meta
        let target_vendor = profile
            .key
            .target_vendor
            .clone()
            .unwrap_or_else(|| TargetVendor::default_for_platform(profile.key.platform));
        let target_env = profile
            .key
            .target_env
            .clone()
            .or_else(|| TargetEnv::default_for_platform(profile.key.platform));
        let target_env_tag = target_env.map(|value| value.triple_component());
        let import_meta = ImportMeta {
            url: module.uri.clone(),
            path: path.clone(),
            file: path.clone(),
            filename: path.clone(),
            dir: dir.clone(),
            dirname: dir.clone(),
            emit: profile.key.emit,
            platform: profile.key.platform,
            runtime: profile.key.runtime,
            target: ImportMetaTarget {
                family: profile.key.platform.family_tag().to_string(),
                vendor: target_vendor.triple_component(),
                env: target_env_tag.clone(),
                abi: target_env_tag,
                arch: profile
                    .key
                    .target_arch
                    .clone()
                    .map(|value| value.triple_component()),
            },
            debug: profile.key.debug,
            test: profile.key.test,
            env: profile.env.clone(),
        };

        // clone the mutable prepared locals
        let mut tree = base.tree.as_ref().clone();
        let mut symbols = base.symbols.as_ref().clone();
        let types = base.types.as_ref().clone();
        let mut roots = base.roots.as_ref().clone();
        let mut export_assignment = None;
        let module_bindings = base.module_bindings.as_ref().clone();
        let mut module_binding_exports = ModuleBindingExportTable::default();
        let imported_modules = ImportedModuleTable::default();
        let mut exported_symbols = ExportedSymbolTable::default();

        // apply static if decorators before exports are built
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_STATIC_IF);
            self.apply_static_if_decorators(
                module_id,
                profile_id,
                Some(&import_meta),
                &mut tree,
                &mut symbols,
                &types,
                &mut roots,
            )?;
        }

        // build export table from bound declarations
        let dependency_items_by_scope = self.dependency_items_by_scope(&tree);
        let export_items_by_scope = self.export_items_by_scope(&tree, &dependency_items_by_scope);
        let export_assignments_by_scope =
            self.export_assignments_by_scope(module_id, &tree, &export_items_by_scope);
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_EXPORTS);
            self.build_module_exports(
                module,
                &tree,
                &mut symbols,
                &roots,
                base.namespace_scope,
                base.namespace_symbol,
                base.default_symbol,
                base.export_assignment_symbol,
                &mut export_assignment,
                &mut exported_symbols,
                &dependency_items_by_scope,
                &export_items_by_scope,
                &export_assignments_by_scope,
            );
        }
        {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_PREPARE_BINDING_EXPORTS);
            self.build_module_binding_exports(
                module,
                &tree,
                &mut symbols,
                &module_bindings,
                &mut module_binding_exports,
                &dependency_items_by_scope,
                &export_items_by_scope,
                &export_assignments_by_scope,
            );
        }

        // publish the prepared artifact
        let dir = DirPrepared::from_base_with(
            base.as_ref(),
            profile_id,
            tree,
            symbols,
            roots,
            export_assignment,
            module_binding_exports,
            imported_modules,
            exported_symbols,
        );

        self.publish_artifact(artifact_key, dir.clone(), |store, version, payload| {
            store.publish_dir_prepared(version, payload)
        });
        context.store_artifact(&artifact_key, &dir, |compiler, artifact_stamp, dir| {
            compiler.store_dir_prepared_image(revision, module_id, profile_id, artifact_stamp, dir)
        });

        Ok(())
    }

    /// Update the symbol id stored in a dependency item.
    pub(super) fn retype_dependency_item_symbol(
        &self,
        item: DependencyItem,
        symbol_id: LocalSymbolId,
    ) -> DependencyItem {
        // rewrite the stored symbol id when present
        match item {
            DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: _,
            } => DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: Some(symbol_id),
            },
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol: _,
            } => DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol: Some(symbol_id),
            },
            DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol: _,
                target_symbol,
            } => DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol: Some(symbol_id),
                target_symbol,
            },
            DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: _,
                target_symbol,
            } => DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol: Some(symbol_id),
                target_symbol,
            },
            item => item,
        }
    }

    /// Update the target symbol stored in a dependency item.
    pub(super) fn retype_dependency_item_target(
        &self,
        item: DependencyItem,
        target_symbol: GlobalSymbolId,
    ) -> DependencyItem {
        // rewrite the stored target symbol when present
        match item {
            DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol,
                target_symbol: _,
            } => DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol,
                target_symbol,
            },
            DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol,
                target_symbol: _,
            } => DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol,
                target_symbol,
            },
            item => item,
        }
    }

    /// Prepare profile DIR for data/text/binary modules.
    fn resolve_data_module_prepare(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<()> {
        // get base DIR (must exist for parsed data modules)
        let base = self
            .artifact_dir_base(module_id)
            .unwrap_or_else(|| panic!("missing committed base dir artifact for {module_id:?}"));

        // populate default export in exported_symbols
        let default_key_id = self.repository.strings.intern("default");
        let default_key = destack_dir::StaticKey::Name(default_key_id);
        let default_export = Export::local(
            module_id,
            default_key,
            destack_dir::SymbolSpace::Value,
            base.default_symbol,
        );
        let mut exported_symbols = ExportedSymbolTable::default();
        exported_symbols.insert(
            (destack_dir::SymbolSpace::Value, default_key),
            default_export,
        );

        // publish the prepared data-module artifact
        let payload = DirPrepared::from_base_with(
            base.as_ref(),
            profile_id,
            base.tree.as_ref().clone(),
            base.symbols.as_ref().clone(),
            base.roots.as_ref().clone(),
            None,
            ModuleBindingExportTable::default(),
            ImportedModuleTable::default(),
            exported_symbols,
        );

        self.publish_artifact(
            ArtifactKey::DirPrepared {
                module: module_id,
                profile: profile_id,
            },
            payload,
            |store, version, payload| store.publish_dir_prepared(version, payload),
        );

        Ok(())
    }
}
