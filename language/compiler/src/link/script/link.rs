use crate::{Compiler, CompilerError, CompilerResult, LinkError, LinkResult};

use destack_artifact::{EmitFormat, OutputFile, PackageOutput, TargetOutputName};
use destack_source::{FileType, ModuleId};
use destack_workspace::BundleFormat;

use super::ScriptLinker;

impl<'a> ScriptLinker<'a> {
    /// Link one discovered script target.
    pub(crate) fn link_target(&self, root_modules: &[ModuleId]) -> CompilerResult<PackageOutput> {
        self.validate_target()?;

        let plan = self.plan(root_modules)?;
        let mut output_files = Vec::new();

        // script outputs
        output_files.extend(
            self.render_script_graph(&plan)
                .map_err(CompilerError::from)?,
        );

        // stylesheet outputs
        output_files.extend(
            self.render_css_stylesheet_outputs(&plan)
                .map_err(CompilerError::from)?,
        );

        // document outputs
        output_files.extend(
            self.render_html_target_outputs(&plan)
                .map_err(CompilerError::from)?,
        );

        // asset outputs
        output_files.extend(
            self.emit_asset_files(plan.asset_reference_map())
                .map_err(CompilerError::from)?,
        );

        // packaged output
        let mut output = self.package_output(output_files);

        // optional manifest
        if self.target.bundle_output.manifest {
            let manifest = self.build_script_manifest(&output, &plan)?;

            self.compiler.append_manifest_output(
                self.package_dir,
                self.target,
                self.target_name(),
                &mut output,
                manifest,
            )?;
        }

        Ok(output)
    }

    /// Validate the script bundle options used by this target.
    fn validate_target(&self) -> LinkResult<()> {
        // only esm bundle output is implemented so far
        if let Some(format) = self.target.bundle_output.format
            && format != BundleFormat::Esm
        {
            return Err(LinkError::InvalidTarget {
                anchor: self.package_id.into(),
                package: self.package_id,
                target: *self.target_id,
                message: format!(
                    "bundleOutput.format '{}' is not implemented yet",
                    Self::bundle_format_name(format)
                ),
            }
            .into());
        }

        Ok(())
    }

    /// Return the config spelling for one bundle format.
    fn bundle_format_name(format: BundleFormat) -> &'static str {
        match format {
            BundleFormat::Esm => "esm",
            BundleFormat::Iife => "iife",
        }
    }

    /// Build the packaged script output groups for this target.
    pub(crate) fn package_output(&self, files: Vec<OutputFile>) -> PackageOutput {
        let mut outputs = indexmap::IndexMap::new();

        // group linked files by their emitted output role
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

    /// Return the grouped output name for one emitted file.
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
}
