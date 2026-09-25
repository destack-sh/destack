use crate::emit::js;
use tspp_artifact::Script;
use tspp_repository::Target;
use tspp_source::ModuleId;

use super::super::{OutputGraph, OutputId, OutputLayout};
use crate::{JsLinker, LinkError, LinkResult};

impl JsLinker<'_> {
    /// Rewrite one JS module to emitted output paths.
    pub(super) fn rewrite_output_script_module(
        &self,
        output_id: OutputId,
        module_id: ModuleId,
        script: &Script,
        output_graph: &OutputGraph,
        output_layout: &OutputLayout,
        target: &Target,
    ) -> LinkResult<js::Module> {
        let mut module = script.module().clone();
        let roots = module.roots.clone();
        let mut rewritten_roots = Vec::with_capacity(module.roots.len());

        // rewrite each top-level statement independently
        for root in &roots {
            if root.ty != js::NodeType::Statement {
                rewritten_roots.push(*root);
                continue;
            }

            let statement_id = js::LocalNodeId::<js::Statement>::new(root.id);
            let rewritten_statement_roots = self.rewrite_output_statement(
                output_id,
                output_graph,
                output_layout,
                target,
                module_id,
                &mut module,
                statement_id,
            )?;

            rewritten_roots.extend(rewritten_statement_roots);
        }

        let import_call_set = self.compiler.collect_script_dynamic_import_calls(&module);

        // rewrite dynamic imports after the static statement pass
        for import_call_id in import_call_set {
            self.rewrite_output_dynamic_import_call(
                output_id,
                output_graph,
                output_layout,
                target,
                module_id,
                &mut module,
                import_call_id,
            )?;
        }

        module.roots = rewritten_roots;

        Ok(module)
    }

    /// Rewrite one output-linked top-level statement.
    pub(super) fn rewrite_output_statement(
        &self,
        output_id: OutputId,
        output_graph: &OutputGraph,
        output_layout: &OutputLayout,
        target: &Target,
        module_id: ModuleId,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
    ) -> LinkResult<Vec<js::LocalNodeIdAny>> {
        let (specifier, target_module, item_set, has_arguments, is_import) = {
            let statement = module.tree.get(statement_id);

            match statement {
                js::Statement::Import {
                    target: specifier,
                    target_module: Some(target_module),
                    items,
                    attributes,
                    ..
                } => (
                    module.strings.get(*specifier).to_string(),
                    *target_module,
                    items.clone().unwrap_or_default(),
                    attributes.is_some(),
                    true,
                ),
                js::Statement::Export {
                    target: Some(specifier),
                    target_module: Some(target_module),
                    items,
                    ..
                } => (
                    module.strings.get(*specifier).to_string(),
                    *target_module,
                    items.clone(),
                    false,
                    false,
                ),
                _ => return Ok(vec![statement_id.into_any()]),
            }
        };

        let dependency_target = self
            .compiler
            .js_dependency_target(&specifier, Some(target_module));

        // keep external targets untouched
        if !self.should_bundle_js_dependency(
            self.module_anchor_span(module_id)?,
            self.package_id,
            self.target_id,
            target,
            &dependency_target,
        )? {
            return Ok(vec![statement_id.into_any()]);
        }

        // collapse bundled same-output targets to local bindings
        if output_graph.shares_output(module_id, target_module) {
            if is_import {
                return self.rewrite_same_output_import_root(
                    module_id,
                    module,
                    statement_id,
                    &item_set,
                    has_arguments,
                    target_module,
                );
            }

            return self.rewrite_same_output_export_root(
                module_id,
                module,
                statement_id,
                &item_set,
            );
        }

        // keep unresolved bundled references untouched if the target chunk is absent
        let Some(target_output_id) = output_graph.output_id_for_module(target_module) else {
            return Ok(vec![statement_id.into_any()]);
        };
        let rewritten_specifier = self.build_js_output_import_reference(
            target,
            output_id,
            target_output_id,
            output_layout,
            self.package_id,
        )?;
        let rewritten_specifier = module.strings.intern(&rewritten_specifier);
        let statement = module.tree.get_mut(statement_id);

        match statement {
            js::Statement::Import { target, .. } => *target = rewritten_specifier,
            js::Statement::Export { target, .. } => *target = Some(rewritten_specifier),
            _ => {}
        }

        Ok(vec![statement_id.into_any()])
    }

    /// Rewrite one same-output import statement.
    fn rewrite_same_output_import_root(
        &self,
        module_id: ModuleId,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
        item_set: &[js::LocalNodeId<js::DependencyItem>],
        has_arguments: bool,
        target_module: ModuleId,
    ) -> LinkResult<Vec<js::LocalNodeIdAny>> {
        let target_module_ref = self.module(target_module)?;

        // asset wrapper imports become local value bindings
        if !target_module_ref.is_code() {
            if item_set.is_empty() {
                return Ok(Vec::new());
            }

            let replacement = self.resource_import_replacement(
                module,
                statement_id,
                module_id,
                target_module,
                self.target_id,
                self.package_id,
                self.context,
            )?;

            return Ok(replacement
                .into_iter()
                .map(js::LocalNodeId::into_any)
                .collect());
        }

        // reject unsupported import attributes
        if has_arguments {
            return Err(self.invalid_output_statement(
                module_id,
                format!(
                    "bundled same-output import attributes are not supported yet in '{}'",
                    self.target_name()
                ),
            ));
        }
        let profile_id = self.profile_id()?;

        let replacement = self.same_output_import_replacement(
            module,
            statement_id,
            module_id,
            target_module,
            profile_id,
            self.package_id,
        )?;

        Ok(replacement
            .into_iter()
            .map(js::LocalNodeId::into_any)
            .collect())
    }

    /// Rewrite one same-output export statement.
    fn rewrite_same_output_export_root(
        &self,
        module_id: ModuleId,
        module: &mut js::Module,
        statement_id: js::LocalNodeId<js::Statement>,
        item_set: &[js::LocalNodeId<js::DependencyItem>],
    ) -> LinkResult<Vec<js::LocalNodeIdAny>> {
        // reject export forms that need binding rewrites
        if !self
            .compiler
            .can_rewrite_internal_js_reexport(module, item_set)
        {
            return Err(self.invalid_output_statement(
                module_id,
                format!(
                    "bundled same-output re-export rewriting is only implemented for plain named exports in '{}'",
                    self.target_name()
                ),
            ));
        }

        let statement = module.tree.get_mut(statement_id);

        if let js::Statement::Export { target, .. } = statement {
            *target = None;
        }

        Ok(vec![statement_id.into_any()])
    }

    /// Build one invalid output statement error.
    fn invalid_output_statement(&self, module_id: ModuleId, message: String) -> LinkError {
        LinkError::InvalidTarget {
            anchor: module_id.into(),
            package: self.package_id,
            target: *self.target_id,
            message,
        }
    }
}
