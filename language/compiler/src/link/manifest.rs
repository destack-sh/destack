use std::path::Path;

use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{
    BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader,
    DynamicScriptDependencyTarget, EmitFormat, ModuleArtifact, OutputFile, ScriptDependencyKind,
    ScriptDependencyTarget, TargetOutputName,
};
use destack_source::{FileType, ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

use super::layout::{OutputLayout, OutputLocation, OutputReferenceKind};
use super::script::ScriptLinkPlan;

impl Compiler {
    /// Build one public build manifest for one target output.
    pub(crate) fn build_target_manifest(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output: &destack_artifact::PackageOutput,
        script_link_plan: Option<&ScriptLinkPlan>,
    ) -> LinkResult<BuildManifest> {
        let output_layout = OutputLayout::new(package_dir, target);
        let mut files = output
            .outputs
            .iter()
            .flat_map(|(output_name, files)| {
                files.iter().map(|file| {
                    self.build_target_manifest_file(
                        package_dir,
                        root_dir,
                        target_id,
                        package_id,
                        target,
                        &output_layout,
                        *output_name,
                        file,
                        script_link_plan,
                    )
                })
            })
            .collect::<LinkResult<Vec<_>>>()?;

        // stable file order
        files.sort_by(|left, right| left.path.cmp(&right.path));

        let index = if target.emit == EmitFormat::Html {
            output
                .outputs
                .get(&TargetOutputName::Document)
                .and_then(|files| files.first())
                .and_then(|file| self.file_output_location(&output_layout, file))
                .map(|location| output_layout.manifest_path(&location))
        } else {
            None
        };

        Ok(BuildManifest { index, files })
    }

    /// Build one public build manifest file record.
    fn build_target_manifest_file(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output_layout: &OutputLayout<'_>,
        output_name: TargetOutputName,
        file: &OutputFile,
        script_link_plan: Option<&ScriptLinkPlan>,
    ) -> LinkResult<BuildManifestFile> {
        let output_location = self.file_output_location(output_layout, file);
        let path = output_location
            .as_ref()
            .map(|output_location| output_layout.manifest_path(output_location))
            .unwrap_or_else(|| self.package_relative_uri_path(package_dir, &file.uri));
        let mut record = BuildManifestFile {
            path,
            r#type: self.build_manifest_file_type(file),
            loader: self.build_manifest_loader(file),
            name: None,
            input: None,
            is_entry: None,
            is_dynamic_entry: None,
            imports: Vec::new(),
            dynamic_imports: Vec::new(),
        };

        if self.is_manifest_chunk_output(file) {
            self.append_manifest_chunk_metadata(
                package_dir,
                root_dir,
                target_id,
                package_id,
                target,
                output_layout,
                output_name,
                &output_location,
                &mut record,
                script_link_plan,
            )?;
        } else if matches!(file.content.file_type(), FileType::Html) {
            record.is_entry = Some(output_name == TargetOutputName::Document);
        }

        Ok(record)
    }

    /// Append chunk metadata for one manifest file record.
    fn append_manifest_chunk_metadata(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output_layout: &OutputLayout<'_>,
        output_name: TargetOutputName,
        output_location: &Option<OutputLocation>,
        record: &mut BuildManifestFile,
        script_link_plan: Option<&ScriptLinkPlan>,
    ) -> LinkResult<()> {
        let Some(script_link_plan) = script_link_plan else {
            return Ok(());
        };

        // single-file and html chunk metadata
        if matches!(output_name, TargetOutputName::Entry) {
            record.name = Some(target.name.clone());
            record.is_entry = Some(true);
            record.is_dynamic_entry = Some(false);
            record.input = script_link_plan
                .first_entry_module()
                .map(|entry_module| self.package_relative_module_path(package_dir, entry_module));
            record.imports = script_link_plan.external_targets().cloned().collect();
            record.dynamic_imports = script_link_plan.dynamic_targets().cloned().collect();

            return Ok(());
        }

        // preserve-modules chunk metadata
        if !matches!(output_name, TargetOutputName::Module) {
            return Ok(());
        }

        let Some(output_location) = output_location else {
            return Ok(());
        };
        let Some(module_id) = self.find_manifest_module_for_output(
            package_dir,
            root_dir,
            target,
            package_id,
            output_location,
            script_link_plan,
        )?
        else {
            return Ok(());
        };

        record.input = Some(self.package_relative_module_path(package_dir, module_id));
        record.is_entry = Some(script_link_plan.entry_modules().contains(&module_id));
        record.is_dynamic_entry = Some(false);
        record.imports = self.build_manifest_imports_for_module(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output_layout,
            module_id,
        )?;
        record.dynamic_imports = self.build_manifest_dynamic_imports_for_module(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output_layout,
            module_id,
        )?;

        Ok(())
    }

    /// Return one output location for one emitted file when its URI is path based.
    fn file_output_location(
        &self,
        output_layout: &OutputLayout<'_>,
        file: &OutputFile,
    ) -> Option<OutputLocation> {
        file.uri
            .to_path_buf()
            .map(|path| output_layout.output_location(path))
    }

    /// Return the module that emitted one preserve-modules chunk output.
    fn find_manifest_module_for_output(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        package_id: PackageId,
        output_location: &OutputLocation,
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<Option<ModuleId>> {
        for module_id in script_link_plan.modules() {
            let module_output_path = self.resolve_script_module_output_path(
                *module_id,
                target,
                package_dir,
                root_dir,
                package_id,
            )?;

            if module_output_path == output_location.path() {
                return Ok(Some(*module_id));
            }
        }

        Ok(None)
    }

    /// Build the public manifest imports for one preserve-modules chunk.
    fn build_manifest_imports_for_module(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output_layout: &OutputLayout<'_>,
        module_id: ModuleId,
    ) -> LinkResult<Vec<String>> {
        let artifact = self
            .artifacts
            .module_artifact(module_id, target_id)
            .ok_or_else(|| LinkError::Internal {
                package: package_id,
                message: format!(
                    "missing module artifact for module {:?} target '{}'",
                    module_id, target_id.name
                ),
            })?;
        let ModuleArtifact::Script(script) = artifact.as_ref() else {
            return Ok(Vec::new());
        };

        let from_output = output_layout.output_location(self.resolve_script_module_output_path(
            module_id,
            target,
            package_dir,
            root_dir,
            package_id,
        )?);
        let mut imports = Vec::new();

        for dependency in &script.linkage.static_dependencies {
            if dependency.kind == ScriptDependencyKind::Type {
                continue;
            }

            imports.push(self.build_manifest_dependency_reference(
                package_dir,
                root_dir,
                target,
                package_id,
                output_layout,
                &from_output,
                &dependency.target,
            )?);
        }

        Ok(imports)
    }

    /// Build the public manifest dynamic imports for one preserve-modules chunk.
    fn build_manifest_dynamic_imports_for_module(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output_layout: &OutputLayout<'_>,
        module_id: ModuleId,
    ) -> LinkResult<Vec<String>> {
        let artifact = self
            .artifacts
            .module_artifact(module_id, target_id)
            .ok_or_else(|| LinkError::Internal {
                package: package_id,
                message: format!(
                    "missing module artifact for module {:?} target '{}'",
                    module_id, target_id.name
                ),
            })?;
        let ModuleArtifact::Script(script) = artifact.as_ref() else {
            return Ok(Vec::new());
        };

        let from_output = output_layout.output_location(self.resolve_script_module_output_path(
            module_id,
            target,
            package_dir,
            root_dir,
            package_id,
        )?);
        let mut imports = Vec::new();

        for dependency in &script.linkage.dynamic_dependencies {
            match &dependency.target {
                DynamicScriptDependencyTarget::Resolved(dependency_target) => {
                    imports.push(self.build_manifest_dependency_reference(
                        package_dir,
                        root_dir,
                        target,
                        package_id,
                        output_layout,
                        &from_output,
                        dependency_target,
                    )?);
                }
                DynamicScriptDependencyTarget::Opaque => {}
            }
        }

        Ok(imports)
    }

    /// Build the public manifest reference for one dependency edge.
    fn build_manifest_dependency_reference(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        package_id: PackageId,
        output_layout: &OutputLayout<'_>,
        from_output: &OutputLocation,
        dependency_target: &ScriptDependencyTarget,
    ) -> LinkResult<String> {
        match dependency_target {
            ScriptDependencyTarget::Module { module, .. } => {
                let to_output =
                    output_layout.output_location(self.resolve_script_module_output_path(
                        *module,
                        target,
                        package_dir,
                        root_dir,
                        package_id,
                    )?);

                Ok(output_layout.output_reference(
                    from_output,
                    &to_output,
                    OutputReferenceKind::Import,
                ))
            }
            ScriptDependencyTarget::External { specifier } => Ok(specifier.clone()),
        }
    }

    /// Return whether one emitted file should be represented as one manifest chunk.
    fn is_manifest_chunk_output(&self, file: &OutputFile) -> bool {
        matches!(
            file.content.file_type(),
            FileType::JavaScript | FileType::TypeScript
        )
    }

    /// Return the manifest file type for one emitted file.
    fn build_manifest_file_type(&self, file: &OutputFile) -> BuildManifestFileType {
        match file.content.file_type() {
            FileType::JavaScript | FileType::TypeScript => BuildManifestFileType::Chunk,
            FileType::Object | FileType::Wasm => BuildManifestFileType::Binary,
            _ => BuildManifestFileType::Asset,
        }
    }

    /// Return the manifest loader string for one emitted file.
    fn build_manifest_loader(&self, file: &OutputFile) -> BuildManifestLoader {
        match file.content.file_type() {
            FileType::JavaScript => BuildManifestLoader::Js,
            FileType::TypeScript => BuildManifestLoader::Ts,
            FileType::Html => BuildManifestLoader::Html,
            FileType::SourceMap => BuildManifestLoader::Map,
            FileType::Json => BuildManifestLoader::Json,
            FileType::TypeScriptDeclaration => BuildManifestLoader::Dts,
            FileType::Wasm => BuildManifestLoader::Wasm,
            FileType::Object => BuildManifestLoader::Object,
            _ => BuildManifestLoader::Asset,
        }
    }
}
