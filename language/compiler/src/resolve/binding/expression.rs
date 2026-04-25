use destack_core::StringId;
use destack_dir::{
    Expression, ImportSource, ImportTarget, LocalNodeId, LocalScopeId, LocalSymbolId, NodeTree,
    ScalarLiteral, SymbolTable, TypeTable,
};
use destack_source::{NodeSpanList, NodeSpanType, SourcePartKey};

use crate::resolve::binding::ResolvedPathSymbolTargets;
use crate::resolve::binding::cache::{ResolveExpressionCache, ResolvePathCacheKey};
use crate::resolve::dependency::loader::LoaderAttribute;
use crate::{Compiler, ResolveError, ResolveResult};
use destack_artifact::{ExportedSymbolTable, ImportedModuleTable};
use destack_workspace::workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return the static string specifier for one import target expression when one exists.
    fn static_import_target_string(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<StringId> {
        let mut expression_id = expression_id;

        loop {
            let expression = tree.get(expression_id);

            // allow parenthesized wrappers around literal specifiers
            if let Expression::Parenthesized { expression } = expression {
                expression_id = *expression;
                continue;
            }

            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(target),
            } = expression
            else {
                return None;
            };

            return Some(*target);
        }
    }

    /// Resolve an Expression.
    pub(crate) fn resolve_expression(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        imported_modules: &mut ImportedModuleTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        exported_symbols: &mut ExportedSymbolTable,
        expression_id: LocalNodeId<Expression>,
        cache: &mut ResolveExpressionCache,
    ) -> ResolveResult<()> {
        let scope = {
            let scope = symbols.get_scope(expression_id, tree);
            let scope = (scope.0, scope.1.clone(), scope.2);
            if module.language_type.is_declaration() {
                (scope.0, scope.1, destack_dir::LocalScopeMark::end())
            } else {
                scope
            }
        };
        let expression = tree.get(expression_id).clone();
        let module_handle = module;

        let expression: Expression = match expression {
            Expression::UnresolvedImport {
                source,
                kind,
                ref target,
                ref items,
                ref attributes,
                ref arguments,
            } => {
                let target = match target {
                    ImportTarget::String(target) => Some(*target),
                    ImportTarget::Expression { target }
                        if matches!(
                            source,
                            ImportSource::ImportCall | ImportSource::RequireCall
                        ) =>
                    {
                        self.static_import_target_string(tree, *target)
                    }
                    _ => None,
                };

                if let Some(target) = target {
                    let loader_override = match self.loader_from_import_attributes(
                        attributes
                            .as_ref()
                            .map(|attributes| attributes.attributes.as_slice()),
                    ) {
                        LoaderAttribute::None => None,
                        LoaderAttribute::Loader(loader) => Some(loader),
                        LoaderAttribute::InvalidType { value } => {
                            return Err(ResolveError::InvalidImportAttributeType {
                                node: expression_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                                value,
                            });
                        }
                    };
                    let Some(remote_target) = self.resolve_import_maybe(
                        revision,
                        module_handle,
                        imported_modules,
                        profile,
                        expression_id.into_global_any(module.id),
                        source,
                        target,
                        kind,
                        loader_override,
                    )?
                    else {
                        return Ok(());
                    };
                    Expression::Import {
                        source,
                        kind,
                        target,
                        target_module: remote_target,
                        items: items.clone(),
                        attributes: attributes.clone(),
                        arguments: arguments.clone(),
                    }
                } else {
                    expression
                }
            }

            Expression::UnresolvedReExport {
                ref target,
                kind,
                ref items,
                ref attributes,
            } => {
                let loader_override = match self.loader_from_import_attributes(
                    attributes
                        .as_ref()
                        .map(|attributes| attributes.attributes.as_slice()),
                ) {
                    LoaderAttribute::None => None,
                    LoaderAttribute::Loader(loader) => Some(loader),
                    LoaderAttribute::InvalidType { value } => {
                        return Err(ResolveError::InvalidImportAttributeType {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                            value,
                        });
                    }
                };
                let Some(remote_target) = self.resolve_import_maybe(
                    revision,
                    module_handle,
                    imported_modules,
                    profile,
                    expression_id.into_global_any(module.id),
                    ImportSource::ExportStatement,
                    *target,
                    kind,
                    loader_override,
                )?
                else {
                    return Ok(());
                };
                Expression::ReExport {
                    target: *target,
                    target_module: remote_target,
                    kind,
                    items: items.clone(),
                    attributes: attributes.clone(),
                }
            }

            Expression::UnresolvedPath {
                ref path,
                ref generic_arguments,
                space_order,
            } => {
                // clone path data to release immutable tree borrows before resolution
                let path = path.clone();
                let generic_arguments = generic_arguments.clone();
                let generic_arguments = if generic_arguments.is_empty() {
                    None
                } else {
                    Some(generic_arguments)
                };
                let resolve_result = if generic_arguments.is_none() && path.segments.len() == 1 {
                    let name = path.first_segment().expect("path is empty");
                    let module_binding_scope_id =
                        self.module_binding_scope_for_global_expression(tree, expression_id);
                    let cache_key = ResolvePathCacheKey {
                        module_id: module.id,
                        scope_id: scope.0,
                        scope_mark: scope.2,
                        module_binding_scope_id,
                        name,
                        space_order,
                    };

                    // prefer cached scope root resolution for single segment paths
                    if let Some(cached) = cache.path_root(cache_key) {
                        Ok((cached, ResolvedPathSymbolTargets::new()))
                    } else {
                        let resolved = self.resolve_absolute_path(
                            revision,
                            module,
                            namespace_symbol,
                            namespace_scope,
                            global_augmentation_scope,
                            exported_symbols,
                            expression_id,
                            expression_id.into_global_any(module.id),
                            profile,
                            (scope.0, &scope.1, scope.2),
                            &path,
                            generic_arguments.clone(),
                            space_order,
                            symbols,
                            tree,
                            cache,
                        );

                        // cache successful path root resolutions
                        if let Ok((expression, _)) = &resolved {
                            cache.insert_path_root(cache_key, expression.clone());
                        }

                        resolved
                    }
                } else {
                    self.resolve_absolute_path(
                        revision,
                        module,
                        namespace_symbol,
                        namespace_scope,
                        global_augmentation_scope,
                        exported_symbols,
                        expression_id,
                        expression_id.into_global_any(module.id),
                        profile,
                        (scope.0, &scope.1, scope.2),
                        &path,
                        generic_arguments.clone(),
                        space_order,
                        symbols,
                        tree,
                        cache,
                    )
                };

                // resolve the path or report the missing symbol
                match resolve_result {
                    Ok((resolved_expression, segment_targets)) => {
                        // record resolved path segment targets for query consumers
                        let source_id = tree.get_source(expression_id.id);
                        for (index, target_symbol) in segment_targets.into_iter().enumerate() {
                            let segment_index =
                                u16::try_from(index).expect("path segment target index overflow");

                            types.set_symbol_target_for_source_part(
                                SourcePartKey::new(
                                    source_id,
                                    NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
                                ),
                                target_symbol,
                            );
                        }

                        resolved_expression
                    }
                    Err(error) => return Err(error),
                }
            }

            Expression::UnresolvedBreak { target, value } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    profile,
                    expression_id.into_global_any(module.id),
                    (scope.0, &scope.1, scope.2),
                    target,
                    symbols,
                )?;
                Expression::Break {
                    target: Some(target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                    value,
                }
            }

            Expression::UnresolvedContinue { target } => {
                let symbol_id = self.resolve_label_symbol(
                    module,
                    profile,
                    expression_id.into_global_any(module.id),
                    (scope.0, &scope.1, scope.2),
                    target,
                    symbols,
                )?;
                Expression::Continue {
                    target: Some(target),
                    target_symbol: Some(symbol_id.into_global(module.id)),
                }
            }

            _ => return Ok(()),
        };
        *tree.get_mut(expression_id) = expression;

        Ok(())
    }
}
