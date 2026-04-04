use std::path::Path;

use crate::link::OutputLayout;
use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{ScriptArtifact, ScriptDependencyTarget};
use destack_codegen_js::{
    DependencyItem, DependencyKind, DependencyMode, Expression, LocalNodeId, NodeTree, NodeType,
    NodeVisitor, NodeVisitorOptions, ScalarLiteral, ScriptModule, Statement, walk_expression,
    walk_root,
};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

use super::{ScriptOutputGraph, ScriptOutputId, ScriptOutputLayout};

/// One rewrite action for one top-level script statement in linked output.
#[derive(Debug, Clone)]
enum OutputStatementRewriteAction {
    /// Keep the statement as-is.
    Keep,
    /// Drop the statement from the emitted chunk body.
    Drop,
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
        tree: &NodeTree,
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
impl Compiler {
    /// Return whether one internal import can be stripped during script linking.
    pub(super) fn can_strip_internal_script_import(
        &self,
        module: &ScriptModule,
        items: &[LocalNodeId<DependencyItem>],
    ) -> bool {
        items.iter().all(|item_id| {
            let item = module.tree.get(*item_id);

            item.mode == DependencyMode::Item && item.alias.is_none() && item.value.is_none()
        })
    }

    /// Return whether one internal re-export can be rewritten as a local export.
    pub(super) fn can_rewrite_internal_script_reexport(
        &self,
        module: &ScriptModule,
        items: &[LocalNodeId<DependencyItem>],
    ) -> bool {
        items.iter().all(|item_id| {
            let item = module.tree.get(*item_id);

            item.mode == DependencyMode::Item && item.value.is_none()
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
            target: target_id.clone(),
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

    /// Classify one same-output import statement during linked output rewriting.
    fn classify_same_output_import_statement(
        &self,
        module_id: ModuleId,
        module: &ScriptModule,
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

        // reject unsupported import attributes for now
        if has_arguments {
            return Err(self.invalid_output_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled same-output import attributes are not supported yet in '{}'",
                    target_id.name
                ),
            ));
        }

        // reject import forms that need binding rewrites
        if !self.can_strip_internal_script_import(module, items) {
            return Err(self.invalid_output_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled same-output import rewriting is only implemented for plain named imports in '{}'",
                    target_id.name
                ),
            ));
        }

        Ok(OutputStatementRewriteAction::Drop)
    }

    /// Classify one same-output export statement during linked output rewriting.
    fn classify_same_output_export_statement(
        &self,
        module_id: ModuleId,
        module: &ScriptModule,
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
                    target_id.name
                ),
            ));
        }

        Ok(OutputStatementRewriteAction::RewriteExportTargetNone)
    }

    /// Classify one output-linked top-level statement.
    fn classify_output_script_statement(
        &self,
        module_id: ModuleId,
        module: &ScriptModule,
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
                    kind,
                    target: specifier,
                    target_module: Some(target_module),
                    items,
                    attributes,
                    ..
                } => (
                    module.strings.get(*specifier).to_string(),
                    *target_module,
                    *kind == DependencyKind::Type,
                    items.clone().unwrap_or_default(),
                    attributes.is_some(),
                    true,
                ),
                Statement::Export {
                    kind,
                    target: Some(specifier),
                    target_module: Some(target_module),
                    items,
                    ..
                } => (
                    module.strings.get(*specifier).to_string(),
                    *target_module,
                    *kind == DependencyKind::Type,
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
                self.classify_same_output_import_statement(
                    module_id,
                    module,
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
        module: &mut ScriptModule,
        statement_id: LocalNodeId<Statement>,
        action: OutputStatementRewriteAction,
        rewritten_specifier: Option<String>,
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
            }
            OutputStatementRewriteAction::Drop => {}
            OutputStatementRewriteAction::RewriteExportTargetNone => {
                let statement = module.tree.get_mut(statement_id);

                if let Statement::Export { target, .. } = statement {
                    *target = None;
                }
            }
        }
    }

    /// Collect all dynamic import call expressions in one script module.
    fn collect_script_dynamic_import_calls(
        &self,
        module: &ScriptModule,
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
        module: &mut ScriptModule,
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
                    target_id.name
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
        script: &ScriptArtifact,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptModule> {
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
                &module,
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
                statement_id,
                action.clone(),
                rewritten_specifier,
            );

            if !matches!(action, OutputStatementRewriteAction::Drop) {
                rewritten_roots.push(*root);
            }
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
