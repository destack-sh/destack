use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{
    EmitFormat, ModuleOutput, OutputFile, PackageOutput, ScriptOutput, TargetOutputName,
};
use destack_codegen_js::{PrintedScriptModule, Module};
use destack_source::{FileType, ModuleId};
use destack_workspace::config::{BundleMode, Target};
use indexmap::IndexMap;

use super::{
    ScriptLinker, ScriptModuleSet, ScriptOutputGraph, ScriptOutputId, ScriptOutputLayout,
    ScriptOutputNode,
};

#[allow(clippy::too_many_arguments)]
impl<'a> ScriptLinker<'a> {
    /// Load one generated script output for linking.
    pub(crate) fn script_output(&self, module_id: ModuleId) -> LinkResult<ScriptOutput> {
        let artifact = self
            .compiler
            .module_output(self.context, module_id, self.target_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "missing module output for module {:?} target '{}': {error:?}",
                    module_id,
                    self.target_name()
                ),
            })?;

        let ModuleOutput::Script(script) = artifact.as_ref() else {
            return Err(LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "expected script output for module {:?} target '{}'",
                    module_id,
                    self.target_name()
                ),
            });
        };

        Ok(script.as_ref().clone())
    }

    /// Return the emitted file type for one linked script target.
    pub(crate) fn linked_script_file_type(&self) -> LinkResult<FileType> {
        match self.target.emit {
            EmitFormat::Js | EmitFormat::Html => Ok(FileType::JavaScript),
            EmitFormat::Ts => Ok(FileType::TypeScript),
            other => Err(LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("unsupported linked script output: {other:?}"),
            }),
        }
    }

    /// Return one module target for linked script printing.
    pub(crate) fn linked_script_module_target(&self) -> Target {
        let mut module_target = self.target.clone();
        module_target.out_file = None;

        // render HTML targets through the JS module path, then wrap at target level
        if module_target.emit == EmitFormat::Html {
            module_target.emit = EmitFormat::Js;
        }

        module_target
    }

    /// Rewrite one linked script module inside one output graph.
    pub(crate) fn rewrite_script_module(
        &self,
        output_id: ScriptOutputId,
        module_id: ModuleId,
        script: &ScriptOutput,
        module_set: &ScriptModuleSet,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
        target: &Target,
    ) -> LinkResult<Module> {
        match output_graph.bundle_mode() {
            BundleMode::SingleFile => self.rewrite_script_module_for_assembly(
                module_id,
                script,
                module_set,
                target,
                self.target_id,
                self.package_id,
            ),
            BundleMode::Chunked | BundleMode::PreserveModules => self.rewrite_output_script_module(
                output_id,
                module_id,
                script,
                output_graph,
                output_layout,
                target,
                self.target_id,
                self.package_id,
            ),
        }
    }

    /// Build one linked script text for one output node.
    pub(crate) fn render_script_output_parts(
        &self,
        output_id: ScriptOutputId,
        output: &ScriptOutputNode,
        module_set: &ScriptModuleSet,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
        target: &Target,
        module_target: &Target,
        file_type: FileType,
    ) -> LinkResult<Vec<(ModuleId, PrintedScriptModule)>> {
        let mut segments = Vec::new();

        // render each output member in stable member order
        for module_id in output.modules() {
            let script = self.script_output(*module_id)?;
            let linked_module = self.rewrite_script_module(
                output_id,
                *module_id,
                &script,
                module_set,
                output_graph,
                output_layout,
                target,
            )?;
            let printed = self
                .compiler
                .print_script_module(
                    *module_id,
                    self.target_id,
                    module_target,
                    file_type,
                    &linked_module,
                    self.context,
                )
                .map_err(|message| LinkError::Internal {
                    anchor: (self.package_id).into(),
                package: self.package_id,
                    message,
                })?;

            segments.push((*module_id, printed));
        }

        Ok(segments)
    }

    /// Build the packaged script output groups for this target.
    pub(crate) fn package_output(&self, files: Vec<OutputFile>) -> PackageOutput {
        let mut outputs = IndexMap::new();

        // group linked script files by their emitted output role
        for file in files {
            let output_name = self.target_output_name_for_file(file.content.file_type());

            outputs
                .entry(output_name)
                .or_insert_with(Vec::new)
                .push(file);
        }

        PackageOutput::new(
            self.target.emit,
            Compiler::package_assembly(self.target.assembly),
            outputs,
        )
    }

    /// Return the grouped output name for one emitted script file.
    fn target_output_name_for_file(&self, file_type: FileType) -> TargetOutputName {
        match file_type {
            FileType::TypeScriptDeclaration => TargetOutputName::Types,
            FileType::SourceMap => TargetOutputName::Maps,
            FileType::Html => TargetOutputName::Document,

            // html and single-file script targets publish an entry file
            FileType::JavaScript | FileType::TypeScript => {
                if self.target.emit == EmitFormat::Html || self.target.is_single_file() {
                    TargetOutputName::Entry
                } else {
                    TargetOutputName::Module
                }
            }

            _ => TargetOutputName::Assets,
        }
    }

    /// Build the runtime HTML document output when the target needs one.
    pub(crate) fn render_runtime_document(
        &self,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> LinkResult<Option<OutputFile>> {
        // non-html targets do not emit a runtime document
        if self.target.emit != EmitFormat::Html {
            return Ok(None);
        }

        let _ = output_graph;
        let _ = output_layout;

        Err(LinkError::Internal {
            anchor: (self.package_id).into(),
                package: self.package_id,
            message: "FUGU #Incomplete: ScriptLinker.render_runtime_document".to_string(),
        })
    }
}
