use crate::emit::js;
use tspp_artifact::Script;
use tspp_repository::{ProviderContext, Target};
use tspp_source::{ModuleId, PackageId, ProfileId, Span, TargetId};

use super::super::ModuleSet;
use crate::{Compiler, JsLinker, LinkError, LinkResult};

impl JsLinker<'_> {
    /// Return whether one module is a bundled entry.
    fn is_bundled_entry_module(&self, module_set: &ModuleSet, module_id: ModuleId) -> bool {
        module_set.entry_modules().contains(&module_id)
    }

    /// Return whether one dependency stays bundled.
    fn is_internal_script_dependency(
        &self,
        module_id: ModuleId,
        specifier: &str,
        target_module: Option<ModuleId>,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<bool> {
        let dependency_target = self.compiler.js_dependency_target(specifier, target_module);
        let module = self.module(module_id)?;

        self.should_bundle_js_dependency(
            Span::empty(module.file_id),
            package_id,
            target_id,
            target,
            &dependency_target,
        )
    }

    /// Rewrite one bundled import statement and return the kept root when one remains.
    fn rewrite_import_statement(
        &self,
        script: &mut js::Module,
        module_id: ModuleId,
        statement_id: js::LocalNodeId<js::Statement>,
        specifier: &str,
        target_module: Option<ModuleId>,
        items: &[js::LocalNodeId<js::DependencyItem>],
        has_arguments: bool,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        profile_id: ProfileId,
        context: &dyn ProviderContext,
    ) -> LinkResult<Option<js::LocalNodeIdAny>> {
        let is_internal = self.is_internal_script_dependency(
            module_id,
            specifier,
            target_module,
            target,
            target_id,
            package_id,
        )?;

        // keep external imports untouched
        if !is_internal {
            return Ok(Some(statement_id.into_any()));
        }

        // bundled asset imports become local value bindings
        if let Some(target_module) = target_module {
            let target_module_ref = self.module(target_module)?;

            if !target_module_ref.is_code() {
                if items.is_empty() {
                    return Ok(None);
                }

                let replacement = self.resource_import_replacement(
                    script,
                    statement_id,
                    module_id,
                    target_module,
                    target_id,
                    package_id,
                    context,
                )?;

                return Ok(replacement.first().copied().map(js::LocalNodeId::into_any));
            }
        }

        // reject unsupported import attributes
        if has_arguments {
            return Err(LinkError::InvalidTarget {
                anchor: module_id.into(),
                package: package_id,
                target: *target_id,
                message: format!(
                    "bundled internal import attributes are not supported yet in '{target_id}'"
                ),
            });
        }

        let Some(target_module) = target_module else {
            return Ok(None);
        };
        let replacement = self.same_output_import_replacement(
            script,
            statement_id,
            module_id,
            target_module,
            profile_id,
            package_id,
        )?;

        Ok(replacement.first().copied().map(js::LocalNodeId::into_any))
    }

    /// Rewrite one bundled export statement and return the kept root when one remains.
    fn rewrite_export_statement(
        &self,
        script: &mut js::Module,
        module_id: ModuleId,
        statement_id: js::LocalNodeId<js::Statement>,
        specifier: Option<String>,
        target_module: Option<ModuleId>,
        items: &[js::LocalNodeId<js::DependencyItem>],
        module_set: &ModuleSet,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Option<js::LocalNodeIdAny>> {
        let Some(specifier) = specifier else {
            return Ok(if self.is_bundled_entry_module(module_set, module_id) {
                Some(statement_id.into_any())
            } else {
                None
            });
        };
        let is_internal = self.is_internal_script_dependency(
            module_id,
            &specifier,
            target_module,
            target,
            target_id,
            package_id,
        )?;

        // keep external re-exports untouched
        if !is_internal {
            return Ok(Some(statement_id.into_any()));
        }

        // non-entry modules do not re-export bindings in bundled output
        if !self.is_bundled_entry_module(module_set, module_id) {
            return Ok(None);
        }

        // reject export forms that need binding rewrites
        if !self
            .compiler
            .can_rewrite_internal_js_reexport(script, items)
        {
            return Err(LinkError::InvalidTarget {
                anchor: module_id.into(),
                package: package_id,
                target: *target_id,
                message: format!(
                    "bundled internal re-export rewriting is only implemented for plain named exports in '{target_id}'"
                ),
            });
        }

        let statement = script.tree.get_mut(statement_id);

        if let js::Statement::Export { target, .. } = statement {
            *target = None;
        }

        Ok(Some(statement_id.into_any()))
    }

    /// Rewrite one bundled top-level statement and return the kept root when one remains.
    fn rewrite_statement(
        &self,
        script: &mut js::Module,
        module_id: ModuleId,
        statement_id: js::LocalNodeId<js::Statement>,
        module_set: &ModuleSet,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        profile_id: ProfileId,
        context: &dyn ProviderContext,
    ) -> LinkResult<Option<js::LocalNodeIdAny>> {
        let statement = script.tree.get(statement_id).clone();

        match statement {
            js::Statement::Import {
                target: specifier,
                target_module,
                items,
                attributes,
            } => {
                let specifier = script.strings.get(specifier).to_string();
                let items = items.unwrap_or_default();

                self.rewrite_import_statement(
                    script,
                    module_id,
                    statement_id,
                    &specifier,
                    target_module,
                    &items,
                    attributes.is_some(),
                    target,
                    target_id,
                    package_id,
                    profile_id,
                    context,
                )
            }
            js::Statement::Export {
                target: export_target,
                target_module,
                items,
                attributes: _,
            } => self.rewrite_export_statement(
                script,
                module_id,
                statement_id,
                export_target.map(|target| script.strings.get(target).to_string()),
                target_module,
                &items,
                module_set,
                target,
                target_id,
                package_id,
            ),

            // non-entry bundled exports degrade to plain expressions
            js::Statement::ExportDefault { value } => {
                Ok(if self.is_bundled_entry_module(module_set, module_id) {
                    Some(statement_id.into_any())
                } else {
                    let statement = script.tree.get_mut(statement_id);
                    *statement = js::Statement::Expression { expression: value };

                    Some(statement_id.into_any())
                })
            }

            _ => Ok(Some(statement_id.into_any())),
        }
    }

    /// Rewrite one emitted JS module for bundled output.
    fn rewrite_module(
        &self,
        module_id: ModuleId,
        script: &Script,
        module_set: &ModuleSet,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        profile_id: ProfileId,
        context: &dyn ProviderContext,
    ) -> LinkResult<js::Module> {
        let mut module = script.module().clone();
        let mut rewritten_roots = Vec::with_capacity(module.roots.len());
        let roots = module.roots.clone();

        // rewrite each top-level statement independently
        for root in &roots {
            if root.ty != js::NodeType::Statement {
                rewritten_roots.push(*root);
                continue;
            }

            let statement_id = js::LocalNodeId::<js::Statement>::new(root.id);
            let rewritten_root = self.rewrite_statement(
                &mut module,
                module_id,
                statement_id,
                module_set,
                target,
                target_id,
                package_id,
                profile_id,
                context,
            )?;

            if let Some(rewritten_root) = rewritten_root {
                rewritten_roots.push(rewritten_root);
            }
        }

        module.roots = rewritten_roots;

        Ok(module)
    }
}

impl JsLinker<'_> {
    /// Trim one printed linked JS module for final concatenation.
    pub(crate) fn trim_script_part(&self, text: String) -> String {
        text.trim_end().to_string()
    }

    /// Return the separator inserted before one later linked JS part.
    pub(crate) fn script_part_separator(
        &self,
        previous_part: &str,
        is_minimal: bool,
    ) -> &'static str {
        if is_minimal {
            if previous_part.ends_with(';') {
                ""
            } else {
                ";"
            }
        } else {
            "\n\n"
        }
    }

    /// Compose one final linked JS text from rendered module segments.
    pub(crate) fn compose_script_text(&self, parts: Vec<String>, is_minimal: bool) -> String {
        let parts = parts
            .into_iter()
            .map(|part| self.trim_script_part(part))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if parts.is_empty() {
            return String::new();
        }

        if is_minimal {
            let mut output = String::new();

            for (index, part) in parts.iter().enumerate() {
                if index > 0 {
                    let separator = self.script_part_separator(parts[index - 1].as_str(), true);
                    output.push_str(separator);
                }

                output.push_str(part);
            }

            return output;
        }

        format!("{}\n", parts.join("\n\n"))
    }

    /// Rewrite one emitted JS module for target-level bundled output.
    pub(crate) fn rewrite_script_module(
        &self,
        module_id: ModuleId,
        script: &Script,
        module_set: &ModuleSet,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
        context: &dyn ProviderContext,
    ) -> LinkResult<js::Module> {
        let profile_id = self
            .compiler
            .profile_id_for_target(context.revision(), target_id)
            .map_err(|error| Compiler::link_error(package_id, error))?;

        self.rewrite_module(
            module_id, script, module_set, target, target_id, package_id, profile_id, context,
        )
    }
}
