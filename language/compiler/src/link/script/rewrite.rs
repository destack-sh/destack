use std::path::{Path, PathBuf};

use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{EmitFormat, ScriptArtifact, ScriptDependencyTarget};
use destack_codegen_js::{
    DependencyItem, DependencyMode, Expression, LocalNodeId, NodeType, ScriptModule, Statement,
};
use destack_source::{FileType, ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

use super::super::layout::{OutputLayout, OutputReferenceKind};
use super::plan::ScriptLinkPlan;

/// One rewrite action for a script statement during linking.
#[derive(Debug, Clone)]
enum ScriptStatementAction {
    /// Keep the statement as-is.
    Keep,
    /// Drop the statement from the linked module body.
    Drop,
    /// Rewrite one re-export to a local export.
    RewriteExportTargetNone,
    /// Rewrite one export value into an expression statement.
    RewriteExportValueToExpression(LocalNodeId<Expression>),
    /// Report one unsupported link rewrite case.
    Error(String),
}

impl Compiler {
    /// Return whether one internal import can be stripped during single-file linking.
    fn can_strip_internal_script_import(
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
    fn can_rewrite_internal_script_reexport(
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
    fn script_dependency_target(
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

    /// Rewrite one generated script module for target-level assembly.
    pub(crate) fn rewrite_script_module_for_link(
        &self,
        module_id: ModuleId,
        script: &ScriptArtifact,
        link_plan: &ScriptLinkPlan,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptModule> {
        let mut module = script.module.clone();
        let is_entry_module = link_plan.entry_modules.contains(&module_id);
        let mut rewritten_roots = Vec::with_capacity(module.roots.len());

        // rewrite top-level module linkage for the current assembly mode
        for root in &module.roots {
            if root.ty != NodeType::Statement {
                rewritten_roots.push(*root);
                continue;
            }

            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let action = {
                let statement = module.tree.get(statement_id);

                match statement {
                    Statement::Import {
                        kind,
                        target: specifier,
                        target_module,
                        items,
                        arguments,
                    } => {
                        let specifier = module.strings.get(*specifier);
                        let dependency_target =
                            self.script_dependency_target(&specifier, *target_module);
                        let is_internal = self.should_bundle_script_dependency(
                            self.module_anchor_span(module_id),
                            package_id,
                            target_id,
                            target,
                            &dependency_target,
                        )?;

                        if !is_internal {
                            ScriptStatementAction::Keep
                        } else if *kind == destack_codegen_js::DependencyKind::Type {
                            ScriptStatementAction::Drop
                        } else if arguments.is_some() {
                            ScriptStatementAction::Error(format!(
                                "bundled internal import attributes are not supported yet in '{}'",
                                target_id.name
                            ))
                        } else if !self.can_strip_internal_script_import(&module, items) {
                            ScriptStatementAction::Error(format!(
                                "bundled internal import rewriting is only implemented for plain named imports in '{}'",
                                target_id.name
                            ))
                        } else {
                            ScriptStatementAction::Drop
                        }
                    }
                    Statement::Export {
                        kind,
                        target: Some(specifier),
                        target_module,
                        items,
                    } => {
                        let specifier = module.strings.get(*specifier);
                        let dependency_target =
                            self.script_dependency_target(&specifier, *target_module);
                        let is_internal = self.should_bundle_script_dependency(
                            self.module_anchor_span(module_id),
                            package_id,
                            target_id,
                            target,
                            &dependency_target,
                        )?;

                        if !is_internal {
                            ScriptStatementAction::Keep
                        } else if *kind == destack_codegen_js::DependencyKind::Type {
                            ScriptStatementAction::Drop
                        } else if !is_entry_module {
                            ScriptStatementAction::Drop
                        } else if !self.can_rewrite_internal_script_reexport(&module, items) {
                            ScriptStatementAction::Error(format!(
                                "bundled internal re-export rewriting is only implemented for plain named exports in '{}'",
                                target_id.name
                            ))
                        } else {
                            ScriptStatementAction::RewriteExportTargetNone
                        }
                    }
                    Statement::Export { .. } => {
                        if is_entry_module {
                            ScriptStatementAction::Keep
                        } else {
                            ScriptStatementAction::Drop
                        }
                    }
                    Statement::ExportValue { value } => {
                        if is_entry_module {
                            ScriptStatementAction::Keep
                        } else {
                            ScriptStatementAction::RewriteExportValueToExpression(*value)
                        }
                    }
                    _ => ScriptStatementAction::Keep,
                }
            };

            match action {
                ScriptStatementAction::Keep => rewritten_roots.push(*root),
                ScriptStatementAction::Drop => {}
                ScriptStatementAction::RewriteExportTargetNone => {
                    let statement = module.tree.get_mut(statement_id);

                    if let Statement::Export { target, .. } = statement {
                        *target = None;
                    }

                    rewritten_roots.push(*root);
                }
                ScriptStatementAction::RewriteExportValueToExpression(value) => {
                    let statement = module.tree.get_mut(statement_id);
                    *statement = Statement::Expression { expression: value };
                    rewritten_roots.push(*root);
                }
                ScriptStatementAction::Error(message) => {
                    return Err(LinkError::InvalidTarget {
                        span: self.module_anchor_span(module_id),
                        package: package_id,
                        target: target_id.clone(),
                        message,
                    });
                }
            }
        }

        module.roots = rewritten_roots;

        Ok(module)
    }

    /// Resolve one emitted script module path for preserve-modules style output.
    pub(crate) fn resolve_script_module_output_path(
        &self,
        module_id: ModuleId,
        target: &Target,
        package_dir: &Path,
        root_dir: Option<&Path>,
        package_id: PackageId,
    ) -> LinkResult<PathBuf> {
        let module = self.program.modules.get(module_id);
        let plan =
            destack_codegen_js::plan_module_output(module.as_ref(), target, package_dir, root_dir)
                .map_err(|error| LinkError::Internal {
                    package: package_id,
                    message: format!("failed to plan script module output: {error:?}"),
                })?;
        let file_type = match target.emit {
            EmitFormat::Js | EmitFormat::Html => FileType::JavaScript,
            EmitFormat::Ts => FileType::TypeScript,
            other => {
                return Err(LinkError::Internal {
                    package: package_id,
                    message: format!("unsupported preserve-modules script output: {other:?}"),
                });
            }
        };

        plan.files
            .into_iter()
            .find(|file| file.file_type == file_type)
            .map(|file| file.output_path)
            .ok_or_else(|| LinkError::Internal {
                package: package_id,
                message: format!(
                    "missing planned script output path for module {:?}",
                    module_id
                ),
            })
    }

    /// Rewrite one script module for preserve-modules output paths.
    pub(crate) fn rewrite_script_module_for_output_paths(
        &self,
        module_id: ModuleId,
        script: &ScriptArtifact,
        target: &Target,
        package_id: PackageId,
        package_dir: &Path,
        root_dir: Option<&Path>,
    ) -> LinkResult<ScriptModule> {
        let mut module = script.module.clone();
        let output_layout = OutputLayout::new(package_dir, target);
        let output_path = output_layout.output_location(self.resolve_script_module_output_path(
            module_id,
            target,
            package_dir,
            root_dir,
            package_id,
        )?);

        // rewrite internal static specifiers to their emitted output paths
        for root in &module.roots {
            if root.ty != NodeType::Statement {
                continue;
            }

            let statement_id = LocalNodeId::<Statement>::new(root.id);
            let statement = module.tree.get_mut(statement_id);

            match statement {
                Statement::Import {
                    target: specifier,
                    target_module: Some(target_module),
                    ..
                }
                | Statement::Export {
                    target: Some(specifier),
                    target_module: Some(target_module),
                    ..
                } => {
                    let target_path =
                        output_layout.output_location(self.resolve_script_module_output_path(
                            *target_module,
                            target,
                            package_dir,
                            root_dir,
                            package_id,
                        )?);
                    let rewritten_specifier = output_layout.output_reference(
                        &output_path,
                        &target_path,
                        OutputReferenceKind::Import,
                    );
                    *specifier = module.strings.intern(&rewritten_specifier);
                }
                _ => {}
            }
        }

        Ok(module)
    }
}
