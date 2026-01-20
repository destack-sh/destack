use std::collections::HashSet;

use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Export, Expression, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, LocalTypeId, ModuleTarget, NamespaceExport, NodeTree, StaticKey,
    SymbolSpace, TypeField, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeResult, Compiler};

use super::super::common::ObjectShape;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Declare the module namespace value type from exported values.
    pub(crate) fn declare_module_namespace_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        exported_symbols: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // pick a stable source node for module imports
        let module_dir = module.dir(profile);
        let module_source_id = module_dir
            .roots
            .first()
            .copied()
            .map(LocalNodeId::into_any)
            .unwrap_or(module_dir.anchor_node);

        // register the module namespace value type
        let namespace_symbol = module.dir(profile).namespace_symbol.into_global(module.id);
        let namespace_ty_id = self.build_namespace_type_from_exports(
            module,
            profile,
            tree,
            exported_symbols,
            &module.dir(profile).namespace_exports.read(),
            module_source_id,
            types,
        )?;
        types.set_value_type(namespace_symbol, namespace_ty_id);

        // register module binding namespace value types
        let binding_exports = module.dir(profile).module_binding_exports.read();
        let bindings = module.dir(profile).module_bindings.read();
        for (binding_any_id, exports) in binding_exports.iter() {
            let binding_id = binding_any_id.into_typed::<Declaration>();
            let Declaration::Namespace { descriptor, .. } = tree.get(binding_id) else {
                continue;
            };

            let Some(binding) = bindings
                .iter()
                .find(|binding| binding.declaration == binding_id)
            else {
                continue;
            };

            let binding_namespace_exports =
                self.collect_namespace_exports_in_scope(tree, binding.scope);
            let binding_symbol = descriptor.symbol.into_global(module.id);
            let binding_source_id = binding_id.into_any();
            let binding_ty_id = self.build_namespace_type_from_exports(
                module,
                profile,
                tree,
                &exports.exports,
                &binding_namespace_exports,
                binding_source_id,
                types,
            )?;
            types.set_value_type(binding_symbol, binding_ty_id);
        }

        Ok(())
    }

    /// Build a namespace object type from a set of exports.
    fn build_namespace_type_from_exports(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        exports: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        namespace_exports: &[NamespaceExport],
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // merge direct exports
        let mut shape = ObjectShape::default();
        self.merge_exports_map_into_shape(module, profile, exports, source_id, types, &mut shape)?;

        // merge namespace exports
        let mut visited = HashSet::new();
        for export in namespace_exports {
            if export.kind != DependencyKind::Value {
                continue;
            }

            self.merge_namespace_target_exports(
                module,
                profile,
                tree,
                export.module_id,
                source_id,
                types,
                &mut shape,
                &mut visited,
            )?;
        }

        Ok(types.insert_type_from_any(shape.into_object_type(), source_id))
    }

    /// Merge exports from a map into a namespace shape.
    fn merge_exports_map_into_shape(
        &self,
        module: &Module,
        profile: ProfileId,
        exports: &indexmap::IndexMap<(SymbolSpace, StaticKey), Export>,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
        shape: &mut ObjectShape,
    ) -> AnalyzeResult<()> {
        // collect exported value fields into a namespace shape
        for export in exports.values() {
            if export.space != SymbolSpace::Value {
                continue;
            }

            let Some(target_symbol) = export.target.resolved() else {
                continue;
            };

            // resolve or import the value type for the export
            let value_ty_id = if let Some(value_ty_id) = types.get_value_type_id(target_symbol) {
                value_ty_id
            } else if target_symbol.module_id != module.id {
                self.resolve_remote_symbol_value_type(
                    module,
                    profile,
                    source_id,
                    target_symbol,
                    types,
                )?
            } else {
                continue;
            };

            shape.fields.push(TypeField {
                key: export.key,
                ty: value_ty_id,
                is_optional: false,
                is_readonly: true,
            });
        }

        Ok(())
    }

    /// Merge namespace export targets into a namespace shape.
    fn merge_namespace_target_exports(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        target: ModuleTarget,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
        shape: &mut ObjectShape,
        visited: &mut HashSet<ModuleTarget>,
    ) -> AnalyzeResult<()> {
        // skip already visited targets
        if !visited.insert(target) {
            return Ok(());
        }

        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module has declared types
                self.require_analyze_module_declare(module_id, profile)?;

                // load the target module exports
                let target_module = self.program.modules.get(module_id);
                let target_module = target_module.read();
                let target_dir = target_module.dir(profile);

                // merge direct exports
                let exports = target_dir.exported_symbols.read();
                self.merge_exports_map_into_shape(
                    module, profile, &exports, source_id, types, shape,
                )?;

                // merge namespace exports
                let namespace_exports = target_dir.namespace_exports.read();
                for export in namespace_exports.iter() {
                    if export.kind != DependencyKind::Value {
                        continue;
                    }

                    self.merge_namespace_target_exports(
                        module,
                        profile,
                        tree,
                        export.module_id,
                        source_id,
                        types,
                        shape,
                        visited,
                    )?;
                }
            }
            ModuleTarget::Binding(specifier) => {
                // load binding exports for the target specifier
                let dir = module.dir(profile);
                let bindings = dir.module_bindings.read();
                let binding_exports = dir.module_binding_exports.read();
                for binding in bindings
                    .iter()
                    .filter(|binding| binding.specifier == specifier)
                {
                    let Some(exports) = binding_exports.get(&binding.declaration.into_any()) else {
                        continue;
                    };

                    // merge direct exports
                    self.merge_exports_map_into_shape(
                        module,
                        profile,
                        &exports.exports,
                        source_id,
                        types,
                        shape,
                    )?;

                    // merge namespace exports
                    let namespace_exports =
                        self.collect_namespace_exports_in_scope(tree, binding.scope);
                    for export in namespace_exports {
                        if export.kind != DependencyKind::Value {
                            continue;
                        }

                        self.merge_namespace_target_exports(
                            module,
                            profile,
                            tree,
                            export.module_id,
                            source_id,
                            types,
                            shape,
                            visited,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect namespace exports declared within a scope.
    fn collect_namespace_exports_in_scope(
        &self,
        tree: &NodeTree,
        scope_id: LocalScopeId,
    ) -> Vec<NamespaceExport> {
        // prepare the namespace export list
        let mut exports = Vec::new();

        // scan dependency items in the target scope
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            let DependencyItem::Remote {
                mode,
                kind,
                alias,
                target_module,
                ..
            } = tree.get(item_id)
            else {
                continue;
            };

            // only consider export star statements
            if *mode != DependencyMode::Namespace
                || alias.is_some()
                || *kind != DependencyKind::Value
            {
                continue;
            }

            // ensure this dependency belongs to an export expression
            let Some(parent_id) = tree.get_parent(item_id.id) else {
                continue;
            };
            let Ok(parent_id) = parent_id.try_into_typed::<Expression>() else {
                continue;
            };
            if !matches!(
                tree.get(parent_id),
                Expression::Export { .. }
                    | Expression::ReExport { .. }
                    | Expression::UnresolvedReExport { .. }
            ) {
                continue;
            }

            // ensure the dependency is in the requested scope
            let (item_scope_id, _) = tree.get_scope(item_id);
            if item_scope_id != scope_id {
                continue;
            }

            // record the namespace export
            exports.push(NamespaceExport {
                module_id: *target_module,
                kind: *kind,
                item: item_id,
            });
        }

        exports
    }
}
