use crate::{LinkError, LinkResult};

use destack_artifact::{OutputFile, PackageOutput};
use destack_source::ModuleId;
use destack_workspace::{BundleFormat, BundleMode};

use crate::link::{OutputLayout, OutputLocation};

use super::{ScriptLinker, ScriptModuleSet, ScriptOutputGraph, ScriptOutputId, ScriptOutputLayout};

impl<'a> ScriptLinker<'a> {
    /// Link one discovered script target.
    pub(crate) fn link_target(&self, entry_modules: &[ModuleId]) -> LinkResult<PackageOutput> {
        self.validate_target()?;

        let module_ids = self.require_module_artifacts(entry_modules)?;

        self.link(entry_modules, &module_ids)
    }

    /// Link one script target from generated module artifacts.
    pub(crate) fn link(
        &self,
        entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
    ) -> LinkResult<PackageOutput> {
        // linked module and output planning
        let module_set = self.build_script_module_set(entry_modules, module_ids)?;
        let output_graph = self.build_script_output_graph(&module_set)?;
        let output_layout = self.build_script_output_layout(&output_graph)?;

        // output files
        let output_files = if output_graph.bundle_mode() == BundleMode::PreserveModules {
            self.link_module_outputs(module_ids, &module_set, &output_graph, &output_layout)?
        } else {
            self.link_output_graph(&module_set, &output_graph, &output_layout)?
        };

        // package output
        let mut output = self.package_output(output_files);

        // optional manifest
        if self.target.bundle.output.manifest {
            let manifest = self.compiler.build_script_manifest(
                self.package_dir,
                self.target,
                &output,
                &output_graph,
                &output_layout,
            );

            self.compiler.append_manifest_output(
                self.package_dir,
                self.target,
                &mut output,
                manifest,
            )?;
        }

        Ok(output)
    }

    /// Validate the script bundle options used by this target.
    fn validate_target(&self) -> LinkResult<()> {
        // only esm bundle output is implemented so far
        if let Some(format) = self.target.bundle.output.format
            && format != BundleFormat::Esm
        {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: format!(
                    "bundle.output.format '{}' is not implemented yet",
                    Self::bundle_format_name(format)
                ),
            });
        }

        // inline dynamic imports still need explicit runtime handling
        if self.target.bundle.inline_dynamic_imports {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: "bundle.inlineDynamicImports is not implemented yet".to_string(),
            });
        }

        // minified bundle output is not wired yet
        if self.target.bundle.minify.is_enabled() {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: self.target_id.clone(),
                message: "bundle.minify is not implemented yet".to_string(),
            });
        }

        Ok(())
    }

    /// Return the config spelling for one bundle format.
    fn bundle_format_name(format: BundleFormat) -> &'static str {
        match format {
            BundleFormat::Esm => "esm",
            BundleFormat::Cjs => "cjs",
            BundleFormat::Iife => "iife",
            BundleFormat::Umd => "umd",
        }
    }

    /// Link preserve-modules outputs for this target.
    fn link_module_outputs(
        &self,
        module_ids: &[ModuleId],
        module_set: &ScriptModuleSet,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> LinkResult<Vec<OutputFile>> {
        let mut output_files = Vec::new();

        // preserve-modules keeps one artifact-level emit per module
        for module_id in module_ids {
            let output_id = output_graph
                .output_id_for_module(*module_id)
                .unwrap_or_else(|| panic!("missing output id for module {module_id:?}"));
            let script = self.script_artifact(*module_id)?;
            let rewritten_module = self.rewrite_script_module(
                output_id,
                *module_id,
                &script,
                module_set,
                output_graph,
                output_layout,
                self.target,
            )?;
            let module = self.compiler.program.modules.get(*module_id);
            let mut rewritten_artifact = script;
            rewritten_artifact.module = rewritten_module;
            let files = self
                .compiler
                .emit_script_artifact_output(
                    module.as_ref(),
                    &rewritten_artifact,
                    self.target_id,
                    self.target,
                    self.package_dir,
                    self.root_dir,
                )
                .map_err(|message| LinkError::Internal {
                    package: self.package_id,
                    message: format!("failed to emit script artifact: {message}"),
                })?;

            output_files.extend(files);
        }

        Ok(output_files)
    }

    /// Link graph-based outputs for this target.
    fn link_output_graph(
        &self,
        module_set: &ScriptModuleSet,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> LinkResult<Vec<OutputFile>> {
        let file_type = self.linked_script_file_type()?;
        let module_target = self.linked_script_module_target();
        let target_layout = OutputLayout::new(self.package_dir, self.target);
        let mut output_files = Vec::new();

        // linked outputs
        for (output_index, output) in output_graph.outputs().iter().enumerate() {
            let output_id = ScriptOutputId(output_index);
            let placement = output_layout.placement(output_id).unwrap_or_else(|| {
                panic!("missing output placement for output id {}", output_id.0)
            });
            let parts = self.render_script_output_parts(
                output_id,
                output,
                module_set,
                output_graph,
                output_layout,
                self.target,
                &module_target,
                file_type,
            )?;
            let code = self.compiler.compose_linked_script_text(
                parts
                    .iter()
                    .map(|(_, printed)| printed.code.clone())
                    .collect(),
            );
            let source_map = self
                .compiler
                .linked_script_source_map_for_parts(self.package_dir, &parts);
            let source_map_path = self
                .target
                .emits_source_map_output()
                .then(|| target_layout.linked_source_map_location(placement.output_location()));
            let source_map_path = source_map_path
                .as_ref()
                .map(|location: &OutputLocation| location.path());
            let files = self
                .compiler
                .emit_script_text_output(
                    self.target,
                    file_type,
                    placement.output_location().path(),
                    code,
                    Some(source_map),
                    source_map_path,
                )
                .map_err(|message| LinkError::Internal {
                    package: self.package_id,
                    message,
                })?;

            output_files.extend(files);
        }

        // runtime document
        if let Some(document) = self.render_runtime_document(output_graph, output_layout)? {
            output_files.push(document);
        }

        Ok(output_files)
    }
}
