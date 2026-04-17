use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, Export, Expression,
    LocalNodeIdAny, LocalScopeId, LocalTypeId, ModuleTarget, NamespaceExport, NodeTree, StaticKey,
    SymbolSpace, Type, TypeField, TypeLiteral,
};

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
        module_source_id: LocalNodeIdAny,
        namespace_symbol: destack_dir::LocalSymbolId,
        namespace_exports: &[NamespaceExport],
        module_bindings: &[destack_dir::ModuleBinding],
        module_binding_exports: &IndexMap<LocalNodeIdAny, destack_dir::ModuleBindingExports>,
    ) -> AnalyzeResult<()> {
        // collect namespace exports once per scope
        let namespace_exports_by_scope = self.collect_namespace_exports_by_scope(ctx.tree);

        // register the module namespace value type
        let namespace_symbol = namespace_symbol.into_global(ctx.module.id);
        let namespace_ty_id = self.build_namespace_type_from_exports(
            &mut ctx.reborrow(),
            exported_symbols,
            namespace_exports,
            module_source_id,
            module_bindings,
            module_binding_exports,
            &namespace_exports_by_scope,
        )?;
        ctx.types.set_value_type(namespace_symbol, namespace_ty_id);

        // register module binding namespace value types
        let binding_entries = module_binding_exports
            .iter()
            .filter_map(|(binding_any_id, exports)| {
                let binding_id = binding_any_id.into_typed::<Declaration>();
                let binding = module_bindings
                    .iter()
                    .find(|binding| binding.declaration == binding_id)?;
                Some((binding_id, binding.scope, exports.exports.clone()))
            })
            .collect::<Vec<_>>();
        for (binding_id, binding_scope, exports) in binding_entries {
            let Declaration::Namespace(declaration) = ctx.tree.get(binding_id) else {
                continue;
            };
            let binding_namespace_exports = namespace_exports_by_scope
                .get(&binding_scope)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let binding_symbol = declaration.symbol.into_global(ctx.module.id);
            let binding_source_id = binding_id.into_any();
            let binding_ty_id = self.build_namespace_type_from_exports(
                &mut ctx.reborrow(),
                &exports,
                binding_namespace_exports,
                binding_source_id,
                module_bindings,
                module_binding_exports,
                &namespace_exports_by_scope,
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
        module_bindings: &[destack_dir::ModuleBinding],
        module_binding_exports: &IndexMap<LocalNodeIdAny, destack_dir::ModuleBindingExports>,
        namespace_exports_by_scope: &HashMap<LocalScopeId, Vec<NamespaceExport>>,
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
                module_bindings,
                module_binding_exports,
                namespace_exports_by_scope,
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
        module_bindings: &[destack_dir::ModuleBinding],
        module_binding_exports: &IndexMap<LocalNodeIdAny, destack_dir::ModuleBindingExports>,
        namespace_exports_by_scope: &HashMap<LocalScopeId, Vec<NamespaceExport>>,
        visited: &mut HashSet<ModuleTarget>,
    ) -> AnalyzeResult<()> {
        // skip already visited targets
        if !visited.insert(target) {
            return Ok(());
        }

        match target {
            ModuleTarget::Module(module_id) => {
                // ensure the target module has interface surface inference
                let target_dir = self.require_artifact_dir_interface(
                    ctx.compiler_context.revision(),
                    module_id,
                    ctx.profile,
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
                        module_bindings,
                        module_binding_exports,
                        namespace_exports_by_scope,
                        visited,
                    )?;
                }
            }
            ModuleTarget::External(_) => {}
            ModuleTarget::Binding(specifier) => {
                // load binding exports for the target specifier
                let binding_entries = module_bindings
                    .iter()
                    .filter(|binding| binding.specifier == specifier)
                    .filter_map(|binding| {
                        let exports = module_binding_exports
                            .get(&binding.declaration.into_any())
                            .map(|exports| exports.exports.clone())?;
                        Some((binding.scope, exports))
                    })
                    .collect::<Vec<_>>();
                for (binding_scope, exports) in binding_entries {
                    // merge direct exports
                    self.merge_exports_map_into_shape(
                        &mut ctx.reborrow(),
                        &exports,
                        source_id,
                        shape,
                    )?;

                    // merge namespace exports
                    let namespace_exports = namespace_exports_by_scope
                        .get(&binding_scope)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                    for export in namespace_exports {
                        if export.kind != DependencyKind::Value {
                            continue;
                        }

                        self.merge_namespace_target_exports(
                            &mut ctx.reborrow(),
                            export.module_id,
                            source_id,
                            shape,
                            module_bindings,
                            module_binding_exports,
                            namespace_exports_by_scope,
                            visited,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Build namespace exports grouped by local scope.
    fn collect_namespace_exports_by_scope(
        &self,
        tree: &NodeTree,
    ) -> HashMap<LocalScopeId, Vec<NamespaceExport>> {
        // prepare the namespace export table
        let mut exports_by_scope = HashMap::new();

        // scan dependency items once for all scopes
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

            // record the namespace export for its scope
            let (item_scope_id, _) = tree.get_scope(item_id);
            if let Some(target_module) = target_module.for_kind(*kind) {
                exports_by_scope
                    .entry(item_scope_id)
                    .or_insert_with(Vec::new)
                    .push(NamespaceExport {
                        module_id: target_module,
                        kind: *kind,
                        item: item_id,
                    });
            }
        }

        exports_by_scope
    }
}
