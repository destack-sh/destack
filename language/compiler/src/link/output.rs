use std::path::Path;

use crate::{Compiler, LinkResult};

use destack_artifact::{
    EmitFormat, OutputContent, OutputFile, PackageAssembly, PackageOutput, TargetOutputName,
};
use destack_source::{FileType, ModuleId, PackageId, Uri};
use destack_workspace::{BundleMode, Target, TargetId};
use indexmap::IndexMap;

use super::assembly::{BinaryTargetAssembly, ScriptTargetAssembly, TargetAssembly};
use super::layout::OutputLayout;
use super::script::ScriptLinkPlan;

impl Compiler {
    /// Return one package-relative path string when possible.
    pub(crate) fn package_relative_path(&self, package_dir: &Path, path: &Path) -> String {
        let relative = path.strip_prefix(package_dir).unwrap_or(path);

        relative.to_string_lossy().replace('\\', "/")
    }

    /// Return one package-relative URI path when possible.
    pub(crate) fn package_relative_uri_path(&self, package_dir: &Path, uri: &Uri) -> String {
        if let Some(path) = uri.to_path_buf() {
            return self.package_relative_path(package_dir, &path);
        }

        uri.to_string()
    }

    /// Return one stable package-relative module source path when possible.
    pub(crate) fn package_relative_module_path(
        &self,
        package_dir: &Path,
        module_id: ModuleId,
    ) -> String {
        let module = self.program.modules.get(module_id);

        if let Some(path) = &module.path {
            return self.package_relative_path(package_dir, path);
        }

        self.package_relative_uri_path(package_dir, &module.uri)
    }

    /// Return the package assembly mode for one target.
    pub(crate) fn package_assembly(&self, target: &Target) -> PackageAssembly {
        match target.bundle.mode {
            BundleMode::PreserveModules => PackageAssembly::PreserveModules,
            BundleMode::SingleFile => PackageAssembly::SingleFile,
            BundleMode::Chunked => PackageAssembly::Chunked,
        }
    }

    /// Return the primary output group for one target.
    fn primary_target_output_name(&self, target: &Target) -> TargetOutputName {
        match target.emit {
            EmitFormat::Native | EmitFormat::Wasm => TargetOutputName::Binary,
            EmitFormat::Html => TargetOutputName::Document,
            EmitFormat::Js | EmitFormat::Ts => {
                if target.is_single_file() {
                    TargetOutputName::Entry
                } else {
                    TargetOutputName::Module
                }
            }
        }
    }

    /// Return the output group for one linked file entry.
    fn target_output_name_for_entry(
        &self,
        target: &Target,
        file_type: FileType,
    ) -> TargetOutputName {
        match file_type {
            FileType::TypeScriptDeclaration => TargetOutputName::Types,
            FileType::SourceMap => TargetOutputName::Maps,
            FileType::Html => TargetOutputName::Document,
            FileType::Object | FileType::Wasm => TargetOutputName::Binary,
            FileType::JavaScript | FileType::TypeScript => {
                if target.emit == EmitFormat::Html {
                    TargetOutputName::Entry
                } else {
                    self.primary_target_output_name(target)
                }
            }
            _ => TargetOutputName::Assets,
        }
    }

    /// Build one package output from linked entries.
    pub(crate) fn package_output_from_entries(
        &self,
        target: &Target,
        assembly: PackageAssembly,
        files: Vec<OutputFile>,
    ) -> PackageOutput {
        let mut outputs = IndexMap::new();

        // group linked entries by their builtin target output name
        for file in files {
            let output_name = self.target_output_name_for_entry(target, file.content.file_type());
            outputs
                .entry(output_name)
                .or_insert_with(Vec::new)
                .push(file);
        }

        PackageOutput::new(target.emit, assembly, outputs)
    }

    /// Build one package output from an assembled script target.
    pub(crate) fn package_output_from_script_target_assembly(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        assembly: ScriptTargetAssembly,
    ) -> LinkResult<PackageOutput> {
        // emitted files
        let mut output =
            self.package_output_from_entries(target, assembly.assembly, assembly.output_files);

        // optional manifest
        self.append_target_manifest_output(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            &mut output,
            assembly.script_link_plan.as_ref(),
        )?;

        Ok(output)
    }

    /// Build one package output from an assembled binary target.
    pub(crate) fn package_output_from_binary_target_assembly(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        assembly: BinaryTargetAssembly,
    ) -> LinkResult<PackageOutput> {
        // emitted files
        let mut output =
            self.package_output_from_entries(target, assembly.assembly, assembly.output_files);

        // later: binary manifest metadata
        let _ = assembly.binary_link_plan;

        // optional manifest
        self.append_target_manifest_output(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            &mut output,
            None,
        )?;

        Ok(output)
    }

    /// Build one package output from an assembled target.
    pub(crate) fn package_output_from_target_assembly(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        assembly: TargetAssembly,
    ) -> LinkResult<PackageOutput> {
        match assembly {
            TargetAssembly::Script(assembly) => self.package_output_from_script_target_assembly(
                package_dir,
                root_dir,
                target_id,
                package_id,
                target,
                assembly,
            ),
            TargetAssembly::Binary(assembly) => self.package_output_from_binary_target_assembly(
                package_dir,
                root_dir,
                target_id,
                package_id,
                target,
                assembly,
            ),
        }
    }

    /// Append one manifest sidecar when the target requests it.
    pub(crate) fn append_target_manifest_output(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output: &mut PackageOutput,
        script_link_plan: Option<&ScriptLinkPlan>,
    ) -> LinkResult<()> {
        // opt out
        if !target.bundle.output.manifest {
            return Ok(());
        }

        // manifest payload
        let manifest = self.build_target_manifest(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output,
            script_link_plan,
        )?;
        let manifest_content = serde_json::to_string_pretty(&manifest)
            .unwrap_or_else(|_| serde_json::to_string(&manifest).unwrap_or_default());

        // emitted sidecar
        let output_layout = OutputLayout::new(package_dir, target);
        let manifest_path = output_layout.manifest_location();

        output
            .outputs
            .entry(TargetOutputName::Manifest)
            .or_insert_with(Vec::new)
            .push(OutputFile {
                uri: Uri::from_path(manifest_path.path()),
                content: OutputContent::json(
                    manifest_content,
                    serde_json::to_value(manifest).unwrap_or_default(),
                    FileType::Json,
                ),
                source: None,
            });

        Ok(())
    }
}
