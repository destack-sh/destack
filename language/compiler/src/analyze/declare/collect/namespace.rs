use std::collections::HashSet;

use indexmap::IndexMap;

use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Export, Expression, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, LocalTypeId, ModuleTarget, NamespaceExport, NodeTree, StaticKey,
    SymbolSpace, Type, TypeField, TypeLiteral,
};

use crate::analyze::DirReadBoundary;
use crate::{AnalyzeResult, Compiler};

use crate::analyze::common::{ObjectShape, TypeContext};
use crate::analyze::infer::RemoteValueTypeReadDomain;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Declare the module namespace value type from exported values.
    pub(crate) fn collect_module_namespace_value_type(
        &self,
        ctx: &mut TypeContext<'_>,
        exported_symbols: &IndexMap<(SymbolSpace, StaticKey), Export>,
    ) -> AnalyzeResult<()> {
        // pick a stable source node for module imports
        let module_dir = ctx.local_dir();
        let module_source_id = module_dir
            .roots
            .first()
            .copied()
            .map(LocalNodeId::into_any)
            .unwrap_or(module_dir.anchor_node);

        // register the module namespace value type
        let namespace_symbol = ctx.local_dir().namespace_symbol.into_global(ctx.module.id);
        let namespace_exports = ctx.local_dir().namespace_exports.read().clone();
        let namespace_ty_id = self.build_namespace_type_from_exports(
            &mut ctx.reborrow(),
            exported_symbols,
            &namespace_exports,
            module_source_id,
        )?;
        ctx.types.set_value_type(namespace_symbol, namespace_ty_id);

        // register module binding namespace value types
        let binding_exports = ctx.local_dir().module_binding_exports.read().clone();
        let bindings = ctx.local_dir().module_bindings.read().clone();
        for (binding_any_id, exports) in &binding_exports {
            let binding_id = binding_any_id.into_typed::<Declaration>();
            let Declaration::Namespace { descriptor, .. } = ctx.tree.get(binding_id) else {
                continue;
            };

            let Some(binding) = bindings
                .iter()
                .find(|binding| binding.declaration == binding_id)
            else {
                continue;
            };

            let binding_namespace_exports =
                self.collect_namespace_exports_in_scope(ctx.tree, binding.scope);
            let binding_symbol = descriptor.symbol.into_global(ctx.module.id);
            let binding_source_id = binding_id.into_any();
            let binding_ty_id = self.build_namespace_type_from_exports(
                &mut ctx.reborrow(),
                &exports.exports,
                &binding_namespace_exports,
                binding_source_id,
            )?;
            ctx.types.set_value_type(binding_symbol, binding_ty_id);
        }

        Ok(())
    }

    /// Build a namespace object type from a set of exports.
    fn build_namespace_type_from_exports(
        &self,
        ctx: &mut TypeContext<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        namespace_exports: &[NamespaceExport],
        source_id: LocalNodeIdAny,
    ) -> AnalyzeResult<LocalTypeId> {
        // merge direct exports
        let mut shape = ObjectShape::default();
        self.merge_exports_map_into_shape(&mut ctx.reborrow(), exports, source_id, &mut shape)?;

        // merge namespace exports
        let mut visited = HashSet::new();
        for export in namespace_exports {
            if export.kind != DependencyKind::Value {
                continue;
            }

            self.merge_namespace_target_exports(
                &mut ctx.reborrow(),
                export.module_id,
                source_id,
                &mut shape,
                &mut visited,
            )?;
        }

        Ok(ctx
            .types
            .insert_type_from_any(shape.into_object_type(), source_id))
    }

    /// Merge exports from a map into a namespace shape.
    fn merge_exports_map_into_shape(
        &self,
        ctx: &mut TypeContext<'_>,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        source_id: LocalNodeIdAny,
        shape: &mut ObjectShape,
    ) -> AnalyzeResult<()> {
        let mut unknown_value_type_id = None;

        // collect exported value fields into a namespace shape
        for export in exports.values() {
            if export.space != SymbolSpace::Value {
                continue;
            }

            let target_symbol = export.target.resolved().or_else(|| {
                export
                    .symbol
                    .map(|symbol| symbol.into_global(ctx.module.id))
            });
            let Some(target_symbol) = target_symbol else {
                continue;
            };

            // resolve or import the value type for the export
            let value_ty_id = if let Some(value_ty_id) = ctx.types.get_value_type_id(target_symbol)
            {
                value_ty_id
            } else if target_symbol.module_id != ctx.module.id {
                // treat export namespace construction as interface surface inference
                self.resolve_remote_symbol_value_type(
                    &mut ctx.reborrow(),
                    source_id,
                    target_symbol,
                    RemoteValueTypeReadDomain::Surface,
                )?
            } else {
                *unknown_value_type_id.get_or_insert_with(|| {
                    ctx.types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        source_id,
                    )
                })
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
        ctx: &mut TypeContext<'_>,
        target: ModuleTarget,
        source_id: LocalNodeIdAny,
        shape: &mut ObjectShape,
        visited: &mut HashSet<ModuleTarget>,
    ) -> AnalyzeResult<()> {
        // skip already visited targets
        if !visited.insert(target) {
            return Ok(());
        }

        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module has interface surface inference
                let target_dir = self.require_artifact_dir_for_boundary(
                    module_id,
                    ctx.profile,
                    DirReadBoundary::Interface,
                )?;

                // merge direct exports
                self.merge_exports_map_into_shape(
                    &mut ctx.reborrow(),
                    &target_dir.exported_symbols,
                    source_id,
                    shape,
                )?;

                // merge namespace exports
                for export in target_dir.namespace_exports.iter() {
                    if export.kind != DependencyKind::Value {
                        continue;
                    }

                    self.merge_namespace_target_exports(
                        &mut ctx.reborrow(),
                        export.module_id,
                        source_id,
                        shape,
                        visited,
                    )?;
                }
            }
            ModuleTarget::Binding(specifier) => {
                // load binding exports for the target specifier
                let binding_entries = {
                    let dir = ctx.local_dir();
                    let bindings = dir.module_bindings.read().clone();
                    let binding_exports = dir.module_binding_exports.read().clone();
                    bindings
                        .into_iter()
                        .filter(|binding| binding.specifier == specifier)
                        .filter_map(|binding| {
                            let exports = binding_exports
                                .get(&binding.declaration.into_any())
                                .map(|exports| exports.exports.clone())?;
                            Some((binding, exports))
                        })
                        .collect::<Vec<_>>()
                };
                for (binding, exports) in binding_entries.iter() {
                    // merge direct exports
                    self.merge_exports_map_into_shape(
                        &mut ctx.reborrow(),
                        exports,
                        source_id,
                        shape,
                    )?;

                    // merge namespace exports
                    let namespace_exports =
                        self.collect_namespace_exports_in_scope(ctx.tree, binding.scope);
                    for export in namespace_exports {
                        if export.kind != DependencyKind::Value {
                            continue;
                        }

                        self.merge_namespace_target_exports(
                            &mut ctx.reborrow(),
                            export.module_id,
                            source_id,
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
            if let Some(target_module) = target_module.for_kind(*kind) {
                exports.push(NamespaceExport {
                    module_id: target_module,
                    kind: *kind,
                    item: item_id,
                });
            }
        }

        exports
    }
}
