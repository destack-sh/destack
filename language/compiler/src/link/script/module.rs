use std::path::Path;

use crate::link::OutputLayout;
use crate::{LinkError, LinkResult};

use destack_artifact::ScriptOutput;
use destack_codegen_js::{
    DependencyItem, DependencyForm, DependencyBinding, Expression, LocalNodeId, LocalNodeIdAny,
    Tree, NodeType, NodeVisitor, NodeVisitorOptions, ScalarLiteral, Module, Statement,
    walk_expression, walk_root,
};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::Target;

use super::{
    ScriptDependencyTarget, ScriptLinker, ScriptOutputGraph, ScriptOutputId, ScriptOutputLayout,
};

/// One rewrite action for one top-level script statement in linked output.
#[derive(Debug, Clone)]
enum OutputStatementRewriteAction {
    /// Keep the statement as-is.
    Keep,
    /// Drop the statement from the emitted chunk body.
    Drop,
    /// Replace the statement with local statement roots.
    Replace(Vec<LocalNodeId<Statement>>),
    /// Rewrite one re-export to a local export.
    RewriteExportTargetNone,
}

/// One visitor that records dynamic import call expressions.
#[derive(Debug, Default)]
struct DynamicImportCallCollector {
    /// The collected import call expression ids.
    import_calls: Vec<LocalNodeId<Expression>>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl DynamicImportCallCollector {
    /// Finish the collector and return the recorded import call ids.
    fn into_import_calls(self) -> Vec<LocalNodeId<Expression>> {
        self.import_calls
    }
}

impl NodeVisitor for DynamicImportCallCollector {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // collect import calls before visiting nested children
        if matches!(expression, Expression::ImportCall { .. }) {
            self.import_calls.push(id);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> ScriptLinker<'a> {
    /// Return whether one internal import can be stripped during script linking.
    pub(super) fn can_strip_internal_script_import(
        &self,
        module: &Module,
        items: &[LocalNodeId<DependencyItem>],
    ) -> bool {
        items.iter().all(|item_id| {
            let item = module.tree.get(*item_id);

            item.binding == DependencyBinding::Named && item.alias.is_none() && item.value.is_none()
        })
    }

    /// Return whether one internal re-export can be rewritten as a local export.
    pub(super) fn can_rewrite_internal_script_reexport(
        &self,
        module: &Module,
        items: &[LocalNodeId<DependencyItem>],
    ) -> bool {
        items.iter().all(|item_id| {
            let item = module.tree.get(*item_id);

            item.binding == DependencyBinding::Named && item.value.is_none()
        })
    }

    /// Build one resolved script dependency target from statement metadata.
    pub(super) fn script_dependency_target(
        &self,
        specifier: &str,
        target_module: Option<ModuleId>,
    ) -> ScriptDependencyTarget {
        if let Some(module) = target_module {
            return ScriptDependencyTarget::Module {
                module,
                specifier: specifier.to_string(),
            };
        }

        ScriptDependencyTarget::External {
            specifier: specifier.to_string(),
        }
    }

    /// Build one invalid output rewrite error.
    fn invalid_output_rewrite(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
        package_id: PackageId,
        message: String,
    ) -> LinkError {
        LinkError::InvalidTarget {
            anchor: module_id.into(),
            package: package_id,
            target: *target_id,
            message,
        }
    }

    /// Build one relative import reference between two emitted outputs.
    pub(crate) fn build_script_output_import_reference(
        &self,
        target: &Target,
        from_output_id: ScriptOutputId,
        to_output_id: ScriptOutputId,
        script_layout: &ScriptOutputLayout,
    ) -> String {
        let output_layout = OutputLayout::new(Path::new(""), target);
        let from_output_location = script_layout
            .output_location(from_output_id)
            .unwrap_or_else(|| {
                panic!(
                    "missing output placement for import source output id {}",
                    from_output_id.0
                )
            });
        let to_output_location = script_layout
            .output_location(to_output_id)
            .unwrap_or_else(|| {
                panic!(
                    "missing output placement for import target output id {}",
                    to_output_id.0
                )
            });

        output_layout.output_reference(from_output_location, to_output_location)
    }

    /// Rewrite one same-output import statement during linked output rewriting.
    fn rewrite_same_output_import_statement(
        &self,
        module_id: ModuleId,
        module: &mut Module,
        statement_id: LocalNodeId<Statement>,
        target_module: ModuleId,
        is_type_dependency: bool,
        items: &[LocalNodeId<DependencyItem>],
        has_arguments: bool,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<OutputStatementRewriteAction> {
        // erase same-output type-only imports
        if is_type_dependency {
            return Ok(OutputStatementRewriteAction::Drop);
        }

        // plain stylesheet imports are side effect only
        if self.is_plain_stylesheet_module(target_module) && !items.is_empty() {
            return Err(self.invalid_output_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "plain stylesheet imports are side effect only in '{}': use a bare import or an explicit file loader for a URL value",
                    self.target_name()
                ),
            ));
        }

        // same-output resource imports become local value bindings
        if !self.module(target_module).is_code() {
            if items.is_empty() {
                return Ok(OutputStatementRewriteAction::Drop);
            }

            let replacement = self.compiler.resource_import_replacement(
                module,
                statement_id,
                module_id,
                target_module,
                target_id,
                package_id,
                self.context,
            )?;

            return Ok(OutputStatementRewriteAction::Replace(replacement));
        }

        // reject unsupported import attributes for now
        if has_arguments {
            return Err(self.invalid_output_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled same-output import attributes are not supported yet in '{}'",
                    self.target_name()
                ),
            ));
        }

        // same-output code imports rewrite through local bindings
        let profile_id = self.profile_id_for_module(module_id)?;
        let replacement = self.compiler.same_output_import_replacement(
            module,
            statement_id,
            module_id,
            target_module,
            profile_id,
            self.target,
            target_id,
            package_id,
            self.context,
        )?;

        Ok(OutputStatementRewriteAction::Replace(replacement))
    }

    /// Classify one same-output export statement during linked output rewriting.
    fn classify_same_output_export_statement(
        &self,
        module_id: ModuleId,
        module: &mut Module,
        is_type_dependency: bool,
        items: &[LocalNodeId<DependencyItem>],
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<OutputStatementRewriteAction> {
        // erase same-output type-only re-exports
        if is_type_dependency {
            return Ok(OutputStatementRewriteAction::Drop);
        }

        // reject export forms that need binding rewrites
        if !self.can_rewrite_internal_script_reexport(module, items) {
            return Err(self.invalid_output_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled same-output re-export rewriting is only implemented for plain named exports in '{}'",
                    self.target_name()
                ),
            ));
        }

        Ok(OutputStatementRewriteAction::RewriteExportTargetNone)
    }

    /// Classify one output-linked top-level statement.
    fn classify_output_script_statement(
        &self,
        module_id: ModuleId,
        module: &Module,
        statement_id: LocalNodeId<Statement>,
        output_id: ScriptOutputId,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<(OutputStatementRewriteAction, Option<String>)> {
        let (specifier, target_module, is_type_dependency, items, has_arguments, is_import) = {
            let statement = module.tree.get(statement_id);

            match statement {
                Statement::Import {
                    form,
                    target: specifier,
                    target_module: Some(target_module),
                    items,
                    attributes,
                    ..
                } => (
                    module.strings.get(*specifier).to_string(),
                    *target_module,
                    *form == DependencyForm::Type,
                    items.clone().unwrap_or_default(),
                    attributes.is_some(),
                    true,
                ),
                Statement::Export {
                    form,
                    target: Some(specifier),
                    target_module: Some(target_module),
                    items,
                    ..
                } => (
                    module.strings.get(*specifier).to_string(),
                    *target_module,
                    *form == DependencyForm::Type,
                    items.clone(),
                    false,
                    false,
                ),
                _ => return Ok((OutputStatementRewriteAction::Keep, None)),
            }
        };

        let dependency_target = self.script_dependency_target(&specifier, Some(target_module));

        // keep external targets untouched
        if !self.should_bundle_script_dependency(
            self.module_anchor_span(module_id),
            package_id,
            target_id,
            target,
            &dependency_target,
        )? {
            return Ok((OutputStatementRewriteAction::Keep, None));
        }

        // collapse bundled same-output targets to local bindings
        if output_graph.shares_output(module_id, target_module) {
            let action = if is_import {
                self.rewrite_same_output_import_statement(
                    module_id,
                    module,
                    statement_id,
                    target_module,
                    is_type_dependency,
                    &items,
                    has_arguments,
                    target_id,
                    package_id,
                )?
            } else {
                self.classify_same_output_export_statement(
                    module_id,
                    module,
                    is_type_dependency,
                    &items,
                    target_id,
                    package_id,
                )?
            };

            return Ok((action, None));
        }

        // keep unresolved bundled references untouched if the target chunk is absent
        let Some(target_output_id) = output_graph.output_id_for_module(target_module) else {
            return Ok((OutputStatementRewriteAction::Keep, None));
        };
        let rewritten_specifier = self.build_script_output_import_reference(
            target,
            output_id,
            target_output_id,
            output_layout,
        );

        Ok((
            OutputStatementRewriteAction::Keep,
            Some(rewritten_specifier),
        ))
    }

    /// Apply one output-linked rewrite action to one statement root.
    fn apply_output_script_statement_rewrite(
        &self,
        module: &mut Module,
        root: LocalNodeIdAny,
        statement_id: LocalNodeId<Statement>,
        action: OutputStatementRewriteAction,
        rewritten_specifier: Option<String>,
        rewritten_roots: &mut Vec<LocalNodeIdAny>,
    ) {
        match action {
            OutputStatementRewriteAction::Keep => {
                if let Some(rewritten_specifier) = rewritten_specifier {
                    let rewritten_specifier = module.strings.intern(&rewritten_specifier);
                    let statement = module.tree.get_mut(statement_id);

                    match statement {
                        Statement::Import { target, .. } => *target = rewritten_specifier,
                        Statement::Export { target, .. } => *target = Some(rewritten_specifier),
                        _ => {}
                    }
                }

                rewritten_roots.push(root);
            }
            OutputStatementRewriteAction::Drop => {}
            OutputStatementRewriteAction::Replace(replacement) => {
                rewritten_roots.extend(replacement.into_iter().map(LocalNodeId::into_any));
            }
            OutputStatementRewriteAction::RewriteExportTargetNone => {
                let statement = module.tree.get_mut(statement_id);

                if let Statement::Export { target, .. } = statement {
                    *target = None;
                }

                rewritten_roots.push(root);
            }
        }
    }

    /// Collect all dynamic import call expressions in one script module.
    fn collect_script_dynamic_import_calls(
        &self,
        module: &Module,
    ) -> Vec<LocalNodeId<Expression>> {
        let mut collector = DynamicImportCallCollector::default();

        // walk every root through the normal visitor entry points
        for root in &module.roots {
            walk_root(&mut collector, &module.tree, root);
        }

        collector.into_import_calls()
    }

    /// Rewrite one dynamic import call to its output specifier when bundled.
    fn rewrite_output_dynamic_import_call(
        &self,
        module_id: ModuleId,
        module: &mut Module,
        import_call_id: LocalNodeId<Expression>,
        output_id: ScriptOutputId,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<()> {
        let (target_expression_id, target_module) = {
            let expression = module.tree.get(import_call_id);
            let Expression::ImportCall {
                target,
                target_module,
                ..
            } = expression
            else {
                return Ok(());
            };

            (*target, *target_module)
        };

        // unresolved and opaque imports stay untouched
        let Some(target_module) = target_module else {
            return Ok(());
        };

        // same-output dynamic imports need one local namespace promise bridge
        if output_graph.shares_output(module_id, target_module) {
            return Err(self.invalid_output_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled same-output dynamic imports are not supported yet in '{}'",
                    self.target_name()
                ),
            ));
        }

        let specifier = {
            let expression = module.tree.get(target_expression_id);
            let Expression::ScalarLiteral {
                value: ScalarLiteral::String(specifier),
            } = expression
            else {
                return Ok(());
            };

            module.strings.get(*specifier).to_string()
        };
        let dependency_target = self.script_dependency_target(&specifier, Some(target_module));

        // only bundled internal imports rewrite to chunk references
        if !self.should_bundle_script_dependency(
            self.module_anchor_span(module_id),
            package_id,
            target_id,
            target,
            &dependency_target,
        )? {
            return Ok(());
        }

        let Some(target_output_id) = output_graph.output_id_for_module(target_module) else {
            return Ok(());
        };
        let rewritten_specifier = self.build_script_output_import_reference(
            target,
            output_id,
            target_output_id,
            output_layout,
        );
        let expression = module.tree.get_mut(target_expression_id);
        let Expression::ScalarLiteral {
            value: ScalarLiteral::String(specifier),
        } = expression
        else {
            return Ok(());
        };

        *specifier = module.strings.intern(&rewritten_specifier);

        Ok(())
    }

    /// Rewrite one script module to emitted output paths.
    pub(crate) fn rewrite_output_script_module(
        &self,
        output_id: ScriptOutputId,
        module_id: ModuleId,
        script: &ScriptOutput,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Module> {
        let mut module = script.module.clone();
        let roots = module.roots.clone();
        let mut rewritten_roots = Vec::with_capacity(module.roots.len());

        // rewrite each top-level statement independently
        for root in &roots {
            if root.ty != NodeType::Statement {
                rewritten_roots.push(*root);
                continue;
            }

            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let (action, rewritten_specifier) = self.classify_output_script_statement(
                module_id,
                &mut module,
                statement_id,
                output_id,
                output_graph,
                output_layout,
                target,
                target_id,
                package_id,
            )?;

            self.apply_output_script_statement_rewrite(
                &mut module,
                *root,
                statement_id,
                action,
                rewritten_specifier,
                &mut rewritten_roots,
            );
        }

        let import_calls = self.collect_script_dynamic_import_calls(&module);

        // rewrite dynamic imports after the static statement pass
        for import_call_id in import_calls {
            self.rewrite_output_dynamic_import_call(
                module_id,
                &mut module,
                import_call_id,
                output_id,
                output_graph,
                output_layout,
                target,
                target_id,
                package_id,
            )?;
        }

        module.roots = rewritten_roots;

        Ok(module)
    }
}
