use destack_artifact::ScriptOutput;
use destack_codegen_js::{
    DependencyItem, DependencyForm, Expression, LocalNodeId, LocalNodeIdAny, NodeType,
    Module, Statement,
};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::Target;

use crate::{LinkError, LinkResult};

use super::{ScriptLinker, ScriptModuleSet};

/// One rewrite action for bundled script statements.
#[derive(Debug, Clone)]
enum ScriptStatementAction {
    /// Keep the statement as-is.
    Keep,
    /// Drop the statement from the bundled module body.
    Drop,
    /// Replace the statement with local statement roots.
    Replace(Vec<LocalNodeId<Statement>>),
    /// Rewrite one re-export to a local export.
    RewriteExportTargetNone,
    /// Rewrite one export value into an expression statement.
    RewriteExportValueToExpression(LocalNodeId<Expression>),
}

impl<'a> ScriptLinker<'a> {
    /// Build one invalid bundled rewrite error.
    fn invalid_bundled_rewrite(
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

    /// Rewrite one bundled import statement.
    fn rewrite_bundled_import_statement(
        &self,
        module_id: ModuleId,
        module: &mut Module,
        statement_id: LocalNodeId<Statement>,
        form: DependencyForm,
        specifier: &str,
        target_module: Option<ModuleId>,
        items: &[LocalNodeId<DependencyItem>],
        has_arguments: bool,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptStatementAction> {
        let dependency_target = self.script_dependency_target(specifier, target_module);
        let is_internal = self.should_bundle_script_dependency(
            self.module_anchor_span(module_id)?,
            package_id,
            target_id,
            target,
            &dependency_target,
        )?;

        // keep external imports untouched
        if !is_internal {
            return Ok(ScriptStatementAction::Keep);
        }

        // erase bundled type-only imports
        if form == DependencyForm::Type {
            return Ok(ScriptStatementAction::Drop);
        }

        // bundled resource imports become local value bindings
        if let Some(target_module) = target_module {
            if !self.module(target_module)?.is_code() {
                if items.is_empty() {
                    return Ok(ScriptStatementAction::Drop);
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

                return Ok(ScriptStatementAction::Replace(replacement));
            }
        }

        // reject unsupported import attributes for now
        if has_arguments {
            return Err(self.invalid_bundled_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled internal import attributes are not supported yet in '{}'",
                    self.target_name()
                ),
            ));
        }

        // unresolved internal imports should have been resolved to modules
        let Some(target_module) = target_module else {
            return Ok(ScriptStatementAction::Drop);
        };

        // bundled code imports rewrite through local bindings
        let profile_id = self.profile_id_for_module(module_id)?;
        let replacement = self.compiler.same_output_import_replacement(
            module,
            statement_id,
            module_id,
            target_module,
            profile_id,
            target,
            target_id,
            package_id,
            self.context,
        )?;

        Ok(ScriptStatementAction::Replace(replacement))
    }

    /// Classify one bundled export statement.
    fn classify_bundled_export_statement(
        &self,
        module_id: ModuleId,
        module: &Module,
        form: DependencyForm,
        specifier: Option<String>,
        target_module: Option<ModuleId>,
        items: &[LocalNodeId<DependencyItem>],
        is_entry_module: bool,
        target_config: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptStatementAction> {
        let Some(specifier) = specifier else {
            return Ok(if is_entry_module {
                ScriptStatementAction::Keep
            } else {
                ScriptStatementAction::Drop
            });
        };

        let dependency_target = self.script_dependency_target(&specifier, target_module);
        let is_internal = self.should_bundle_script_dependency(
            self.module_anchor_span(module_id)?,
            package_id,
            target_id,
            target_config,
            &dependency_target,
        )?;

        // keep external re-exports untouched
        if !is_internal {
            return Ok(ScriptStatementAction::Keep);
        }

        // erase bundled type-only re-exports
        if form == DependencyForm::Type {
            return Ok(ScriptStatementAction::Drop);
        }

        // non-entry modules do not re-export bindings in bundled output
        if !is_entry_module {
            return Ok(ScriptStatementAction::Drop);
        }

        // reject export forms that need binding rewrites
        if !self.can_rewrite_internal_script_reexport(module, items) {
            return Err(self.invalid_bundled_rewrite(
                module_id,
                target_id,
                package_id,
                format!(
                    "bundled internal re-export rewriting is only implemented for plain named exports in '{}'",
                    self.target_name()
                ),
            ));
        }

        Ok(ScriptStatementAction::RewriteExportTargetNone)
    }

    /// Classify one bundled top-level statement.
    fn classify_bundled_script_statement(
        &self,
        module_id: ModuleId,
        module: &mut Module,
        statement_id: LocalNodeId<Statement>,
        is_entry_module: bool,
        target_config: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptStatementAction> {
        let statement = module.tree.get(statement_id);

        match statement {
            Statement::Import {
                form,
                target: specifier,
                target_module,
                items,
                attributes,
            } => self.rewrite_bundled_import_statement(
                module_id,
                module,
                statement_id,
                *form,
                module.strings.get(*specifier),
                *target_module,
                items.as_deref().unwrap_or(&[]),
                attributes.is_some(),
                target_config,
                target_id,
                package_id,
            ),
            Statement::Export {
                form,
                target: export_target,
                target_module,
                items,
                attributes: _,
            } => self.classify_bundled_export_statement(
                module_id,
                module,
                *form,
                export_target.map(|target| module.strings.get(target).to_string()),
                *target_module,
                items,
                is_entry_module,
                target_config,
                target_id,
                package_id,
            ),
            Statement::ExportValue { value } => Ok(if is_entry_module {
                ScriptStatementAction::Keep
            } else {
                ScriptStatementAction::RewriteExportValueToExpression(*value)
            }),
            _ => Ok(ScriptStatementAction::Keep),
        }
    }

    /// Apply one bundled rewrite action to one statement root.
    fn apply_bundled_script_statement_action(
        &self,
        module: &mut Module,
        root: LocalNodeIdAny,
        statement_id: LocalNodeId<Statement>,
        action: ScriptStatementAction,
        rewritten_roots: &mut Vec<LocalNodeIdAny>,
    ) -> LinkResult<()> {
        match action {
            ScriptStatementAction::Keep => rewritten_roots.push(root),
            ScriptStatementAction::Drop => {}
            ScriptStatementAction::Replace(replacement) => {
                rewritten_roots.extend(replacement.into_iter().map(LocalNodeId::into_any));
            }
            ScriptStatementAction::RewriteExportTargetNone => {
                let statement = module.tree.get_mut(statement_id);

                if let Statement::Export { target, .. } = statement {
                    *target = None;
                }

                rewritten_roots.push(root);
            }
            ScriptStatementAction::RewriteExportValueToExpression(value) => {
                let statement = module.tree.get_mut(statement_id);
                *statement = Statement::Expression { expression: value };
                rewritten_roots.push(root);
            }
        }

        Ok(())
    }

    /// Normalize one printed linked script module for final concatenation.
    pub(super) fn normalize_linked_script_part(&self, text: String) -> String {
        text.trim_end().to_string()
    }

    /// Compose one final linked script text from rendered module segments.
    pub(super) fn compose_linked_script_text(&self, parts: Vec<String>) -> String {
        let parts = parts
            .into_iter()
            .map(|part| self.normalize_linked_script_part(part))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if parts.is_empty() {
            return String::new();
        }

        format!("{}\n", parts.join("\n\n"))
    }

    /// Rewrite one generated script module for target-level bundled assembly.
    pub(crate) fn rewrite_script_module_for_assembly(
        &self,
        module_id: ModuleId,
        script: &ScriptOutput,
        module_set: &ScriptModuleSet,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Module> {
        let mut module = script.module.clone();
        let is_entry_module = module_set.entry_modules.contains(&module_id);
        let mut rewritten_roots = Vec::with_capacity(module.roots.len());
        let roots = module.roots.clone();

        // rewrite each top-level statement independently
        for root in &roots {
            if root.ty != NodeType::Statement {
                rewritten_roots.push(*root);
                continue;
            }

            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let action = self.classify_bundled_script_statement(
                module_id,
                &mut module,
                statement_id,
                is_entry_module,
                target,
                target_id,
                package_id,
            )?;

            self.apply_bundled_script_statement_action(
                &mut module,
                *root,
                statement_id,
                action,
                &mut rewritten_roots,
            )?;
        }

        module.roots = rewritten_roots;

        Ok(module)
    }
}
