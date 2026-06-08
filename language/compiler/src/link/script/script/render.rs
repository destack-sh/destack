use crate::{Compiler, LinkError, LinkResult};
use destack_artifact::OutputFile;
use destack_codegen_js as js;
use destack_repository::BundleMode;
use destack_source::{FileType, ModuleId};

use super::super::plan::Plan;
use super::super::{OutputId, ScriptLinker};
use super::linker::OutputModule;
use crate::link::{OutputLocation, TargetLocation};
use destack_artifact::ScriptOutput;

impl<'a> ScriptLinker<'a> {
    /// Build one linked script text for one output node.
    pub(crate) fn render_script_output_parts(
        &self,
        output_id: OutputId,
        plan: &Plan,
        file_type: FileType,
    ) -> LinkResult<Vec<(ModuleId, js::PrintedScriptModule)>> {
        let output = plan
            .output_graph()
            .output(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "missing script output graph node for output id {}",
                    output_id.0
                ),
            })?;
        let mut modules = Vec::<OutputModule>::new();

        // rewrite each output member in stable member order
        for module_id in output.modules() {
            let script = self.script_output_for_output(output_id, *module_id, plan)?;

            modules.push((*module_id, script.module));
        }

        // normalize output-local imports before output-level minification and printing
        self.rewrite_output_script_imports(&mut modules)?;
        self.rewrite_module_defaults(&mut modules)?;

        // syntax minification
        if self.target.should_minify_bundle_script_syntax() {
            self.minify_output_syntax(&mut modules)?;
        }

        // identifier minification
        if self.target.minify.identifiers {
            self.minify_output_identifiers(&mut modules)?;
        }

        let mut segments = Vec::with_capacity(modules.len());

        // print each rewritten module after output-level rewrites and minification
        for (module_id, module) in modules {
            let printed = self
                .compiler
                .print_script_module(module_id, self.target, file_type, &module, self.context)
                .map_err(|error| Compiler::link_error(self.package_id, error))?;

            segments.push((module_id, printed));
        }

        Ok(segments)
    }

    /// Render the script outputs for the current script graph.
    pub(in super::super) fn render_script_graph(&self, plan: &Plan) -> LinkResult<Vec<OutputFile>> {
        self.validate_script_print_format()?;

        if plan.output_graph().bundle_mode() == BundleMode::PreserveModules {
            return self.link_module_outputs(plan);
        }

        self.link_output_graph(plan)
    }

    /// Link preserve-modules outputs for this target.
    fn link_module_outputs(&self, plan: &Plan) -> LinkResult<Vec<OutputFile>> {
        let mut output_files = Vec::new();

        // preserve-modules keeps one artifact-level output per module
        for module_id in plan.module_set().modules() {
            let output_id = plan
                .output_graph()
                .output_id_for_module(*module_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!("missing output id for script module {:?}", module_id),
                })?;
            let script = self.script_output_for_output(output_id, *module_id, plan)?;
            let module = self.module(*module_id)?;

            let files = self
                .compiler
                .link_script_output_files(
                    module.as_ref(),
                    &script,
                    self.target,
                    self.package_dir,
                    self.root_dir,
                    self.context,
                )
                .map_err(|error| Compiler::link_error(self.package_id, error))?;

            output_files.extend(files);
        }

        Ok(output_files)
    }

    /// Return the final linked script output for one output member.
    fn script_output_for_output(
        &self,
        output_id: OutputId,
        module_id: ModuleId,
        plan: &Plan,
    ) -> LinkResult<ScriptOutput> {
        let source_module = self.module(module_id)?;

        // resource modules are synthesized by the linker with final linked values
        if !source_module.is_code() {
            return self.build_resource_script_output(
                output_id,
                module_id,
                plan.output_graph(),
                plan,
            );
        }

        let script = self.script_output(module_id)?;
        let rewritten_module = self.rewrite_code_script_module(
            output_id,
            module_id,
            &script,
            plan.module_set(),
            plan.output_graph(),
            plan.output_layout(),
            self.target,
        )?;
        let mut script = script;
        script.module = rewritten_module;

        Ok(script)
    }

    /// Link graph-based outputs for this target.
    fn link_output_graph(&self, plan: &Plan) -> LinkResult<Vec<OutputFile>> {
        let file_type = self.script_output_file_type()?;
        let target_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut output_files = Vec::new();

        // linked outputs
        for (output_index, _output) in plan.output_graph().outputs().iter().enumerate() {
            let output_id = OutputId(output_index);
            let output_location =
                plan.output_layout()
                    .output_location(output_id)
                    .ok_or_else(|| LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "missing output location for script output id {}",
                            output_id.0
                        ),
                    })?;
            let parts = self.render_script_output_parts(output_id, plan, file_type)?;
            let code = self.compiler.compose_script_text(
                parts
                    .iter()
                    .map(|(_, printed)| printed.code.clone())
                    .collect(),
                self.target.should_minify_bundle_script_output(),
            );
            let source_map_path = self
                .target
                .emits_source_map_output()
                .then(|| target_layout.linked_source_map_location(output_location));
            let source_map_path = source_map_path
                .as_ref()
                .map(|location: &OutputLocation| location.path());
            let emitted_source_map_path = source_map_path.unwrap_or_else(|| output_location.path());
            let source_map = self
                .compiler
                .script_source_map_for_parts(
                    self.package_dir,
                    emitted_source_map_path,
                    &parts,
                    self.target.should_minify_bundle_script_output(),
                    self.context,
                )
                .map_err(|error| Compiler::link_error(self.package_id, error))?;
            let files = self
                .compiler
                .link_script_text_files(
                    self.target,
                    file_type,
                    output_location.path(),
                    code,
                    Some(source_map),
                    source_map_path,
                )
                .map_err(|message| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message,
                })?;

            output_files.extend(files);
        }

        Ok(output_files)
    }
}
