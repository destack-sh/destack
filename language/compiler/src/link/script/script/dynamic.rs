use destack_codegen_js as js;
use destack_source::ModuleId;
use destack_workspace::Target;

use super::super::{OutputGraph, OutputId, OutputLayout};
use crate::{Compiler, LinkError, LinkResult, ScriptLinker};

/// One visitor that records dynamic import call expressions.
#[derive(Debug, Default)]
struct DynamicImportCallCollector {
    /// The collected import call expression ids.
    import_calls: Vec<js::LocalNodeId<js::Expression>>,
    /// Visitor options.
    options: js::NodeVisitorOptions,
}

impl DynamicImportCallCollector {
    /// Finish the collector and return the recorded import call ids.
    fn into_import_calls(self) -> Vec<js::LocalNodeId<js::Expression>> {
        self.import_calls
    }
}

impl js::NodeVisitor for DynamicImportCallCollector {
    fn options(&self) -> &js::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &js::Tree,
        id: js::LocalNodeId<js::Expression>,
        expression: &js::Expression,
    ) {
        // collect import calls before visiting nested children
        if matches!(expression, js::Expression::ImportCall { .. }) {
            self.import_calls.push(id);
        }

        js::walk_expression(self, tree, id, expression);
    }
}

impl Compiler {
    /// Collect all dynamic import call expressions in one script module.
    pub(crate) fn collect_script_dynamic_import_calls(
        &self,
        module: &js::Module,
    ) -> Vec<js::LocalNodeId<js::Expression>> {
        let mut collector = DynamicImportCallCollector::default();

        // walk every root through the normal visitor entry points
        for root in &module.roots {
            js::walk_root(&mut collector, &module.tree, root);
        }

        collector.into_import_calls()
    }
}

impl ScriptLinker<'_> {
    /// Rewrite one dynamic import call to its output specifier when bundled.
    pub(super) fn rewrite_output_dynamic_import_call(
        &self,
        output_id: OutputId,
        output_graph: &OutputGraph,
        output_layout: &OutputLayout,
        target: &Target,
        module_id: ModuleId,
        module: &mut js::Module,
        import_call_id: js::LocalNodeId<js::Expression>,
    ) -> LinkResult<()> {
        let (target_expression_id, target_module) = {
            let expression = module.tree.get(import_call_id);
            let js::Expression::ImportCall {
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
            return Err(LinkError::InvalidTarget {
                anchor: module_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: format!(
                    "bundled same-output dynamic imports are not supported yet in '{}'",
                    self.target_name()
                ),
            });
        }

        let specifier = {
            let expression = module.tree.get(target_expression_id);
            let js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(specifier),
            } = expression
            else {
                return Ok(());
            };

            module.strings.get(*specifier).to_string()
        };
        let dependency_target = self
            .compiler
            .script_dependency_target(&specifier, Some(target_module));

        // only bundled internal imports rewrite to chunk references
        if !self.compiler.should_bundle_script_dependency(
            self.module_anchor_span(module_id),
            self.package_id,
            self.target_id,
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
            self.package_id,
        )?;
        let expression = module.tree.get_mut(target_expression_id);
        let js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(specifier),
        } = expression
        else {
            return Ok(());
        };

        *specifier = module.strings.intern(&rewritten_specifier);

        Ok(())
    }
}
