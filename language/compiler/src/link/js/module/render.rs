use crate::link::{OutputLocation, TargetLocation};
use crate::{Compiler, CompilerError, CompilerResult, LinkError, LinkResult};
use destack_artifact::{BundleFile, Script};
use destack_js as js;
use destack_source::{ModuleId, ProvenanceBuilder, ProvenanceTable, TextMap};

use super::super::plan::Plan;
use super::super::{JsLinker, OutputId, SCRIPT_SEPARATOR};

impl<'a> JsLinker<'a> {
    /// Compose one JavaScript output and its provenance from printed modules.
    pub(crate) fn compose_script(
        &self,
        modules: Vec<js::PrintedModule>,
    ) -> CompilerResult<(String, ProvenanceTable, TextMap)> {
        let mut output = String::new();
        let mut provenance = ProvenanceTable::build();
        let mut map = TextMap::default();

        // append each nonempty module and its mapped extents
        for printed in modules {
            let js::PrintedModule {
                text,
                map: mut module_map,
                provenance: module_provenance,
                ..
            } = printed;
            let text = text.trim_end();
            if text.is_empty() {
                continue;
            }

            if !output.is_empty() {
                output.push_str(SCRIPT_SEPARATOR);
            }

            let remap = provenance.import(&module_provenance);
            module_map.truncate(text.len() as u32);
            map.append(module_map, &remap, output.len() as u32)
                .map_err(|error| CompilerError::Internal {
                    message: error.to_string(),
                })?;
            output.push_str(text);
        }

        // terminate nonempty script output with one newline
        if !output.is_empty() {
            output.push('\n');
        }

        Ok((output, provenance.finish(), map))
    }

    /// Rewrite, name, and print the modules in one JavaScript output.
    pub(crate) fn print_output(
        &self,
        output_id: OutputId,
        plan: &Plan,
    ) -> LinkResult<Vec<js::PrintedModule>> {
        let output = plan
            .output_graph()
            .output(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing JS output graph node for output id {}", output_id.0),
            })?;
        let mut modules = Vec::<(ModuleId, Script)>::new();

        // rewrite each output member in stable member order
        for module_id in output.modules() {
            let script = self.link_module(output_id, *module_id, plan)?;

            modules.push((*module_id, script));
        }

        // assign one collision-free name to each output symbol
        self.assign_output_names(&mut modules);

        let mut printed_modules = Vec::with_capacity(modules.len());

        // print each rewritten module
        for (module_id, script) in modules {
            let printed = self
                .print_js_module(module_id, script.module(), self.context)
                .map_err(|error| Compiler::link_error(self.package_id, error))?;

            printed_modules.push(printed);
        }

        Ok(printed_modules)
    }

    /// Render the JavaScript output graph.
    pub(in super::super) fn render_js_graph(
        &self,
        plan: &Plan,
        provenance: &mut ProvenanceBuilder,
    ) -> LinkResult<Vec<BundleFile>> {
        let target_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut output_files = Vec::new();

        // linked outputs
        for output_index in 0..plan.output_graph().outputs().len() {
            let output_id = OutputId(output_index);
            let output_location =
                plan.output_layout()
                    .output_location(output_id)
                    .ok_or_else(|| LinkError::Internal {
                        anchor: (self.package_id).into(),
                        package: self.package_id,
                        message: format!(
                            "missing output location for JS output id {}",
                            output_id.0
                        ),
                    })?;
            let modules = self.print_output(output_id, plan)?;
            let (code, output_provenance, output_map) = self
                .compose_script(modules)
                .map_err(|error| Compiler::link_error(self.package_id, error))?;
            let source_map_path = self
                .target
                .emits_source_map_output()
                .then(|| target_layout.linked_source_map_location(output_location));
            let source_map_path = source_map_path
                .as_ref()
                .map(|location: &OutputLocation| location.path());
            let emitted_source_map_path = source_map_path.unwrap_or_else(|| output_location.path());
            let source_map = self
                .target
                .emits_source_maps()
                .then(|| {
                    self.script_source_map(
                        self.target,
                        emitted_source_map_path,
                        &output_provenance,
                        &output_map,
                        self.context,
                    )
                })
                .transpose()
                .map_err(|error| Compiler::link_error(self.package_id, error))?;
            let remap = provenance.import(&output_provenance);
            let mut bundle_map = TextMap::default();
            bundle_map
                .append(output_map, &remap, 0)
                .map_err(|error| LinkError::Internal {
                    anchor: self.package_id.into(),
                    package: self.package_id,
                    message: format!("failed to import JavaScript text map: {error}"),
                })?;
            let files = self
                .link_script_text_files(
                    self.target,
                    output_location.path(),
                    code,
                    source_map,
                    source_map_path,
                    bundle_map,
                )
                .map_err(|error| Compiler::link_error(self.package_id, error))?;

            output_files.extend(files);
        }

        Ok(output_files)
    }

    /// Link one module into its planned JavaScript output.
    fn link_module(&self, output: OutputId, module: ModuleId, plan: &Plan) -> LinkResult<Script> {
        let source_module = self.module(module)?;

        // synthesize resource modules with their final linked values
        if !source_module.is_code() {
            return self.build_resource_js_output(output, module, plan.output_graph(), plan);
        }

        let script = self.script(module)?;
        self.rewrite_code_script_module(
            output,
            module,
            &script,
            plan.module_set(),
            plan.output_graph(),
            plan.output_layout(),
        )
    }
}
