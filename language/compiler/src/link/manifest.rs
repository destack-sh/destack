use std::path::Path;

use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{
    BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader,
    DynamicScriptDependencyTarget, EmitFormat, ModuleArtifact, OutputFile, PackageAssembly,
    ScriptDependencyKind, ScriptDependencyTarget, TargetOutputName,
};
use destack_source::{FileType, ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

use super::layout::{OutputLayout, OutputLocation, OutputReferenceKind};
use super::script::ScriptLinkPlan;

/// One internal manifest record before lowering to the public artifact type.
struct ManifestFileRecord {
    /// The manifest-visible relative output path.
    path: String,
    /// The manifest-visible file kind.
    file_type: BuildManifestFileType,
    /// The manifest-visible loader kind.
    loader: BuildManifestLoader,
    /// Optional chunk-style metadata.
    chunk: Option<ManifestChunkMetadata>,
}

/// One manifest chunk metadata payload.
struct ManifestChunkMetadata {
    /// The exposed chunk name.
    name: Option<String>,
    /// The source module path for this chunk.
    input: Option<String>,
    /// Whether this chunk is an entry.
    is_entry: bool,
    /// Whether this chunk is a dynamic entry.
    is_dynamic_entry: bool,
    /// Static import references from this chunk.
    imports: Vec<String>,
    /// Dynamic import references from this chunk.
    dynamic_imports: Vec<String>,
}

impl From<ManifestFileRecord> for BuildManifestFile {
    fn from(record: ManifestFileRecord) -> Self {
        let chunk = record.chunk;

        Self {
            path: record.path,
            r#type: record.file_type,
            loader: record.loader,
            name: chunk.as_ref().and_then(|chunk| chunk.name.clone()),
            input: chunk.as_ref().and_then(|chunk| chunk.input.clone()),
            is_entry: chunk.as_ref().map(|chunk| chunk.is_entry),
            is_dynamic_entry: chunk.as_ref().map(|chunk| chunk.is_dynamic_entry),
            imports: chunk
                .as_ref()
                .map(|chunk| chunk.imports.clone())
                .unwrap_or_default(),
            dynamic_imports: chunk
                .as_ref()
                .map(|chunk| chunk.dynamic_imports.clone())
                .unwrap_or_default(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
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
        // layout
        let output_layout = OutputLayout::new(package_dir, target);

        // records
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

        // html entry index
        let index = self.build_manifest_index_path(&output_layout, target, output);

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
        // path
        let output_location = self.file_output_location(output_layout, file);
        let path = output_location
            .as_ref()
            .map(|output_location| output_layout.manifest_path(output_location))
            .unwrap_or_else(|| self.package_relative_uri_path(package_dir, &file.uri));

        // chunk metadata
        let chunk = self.build_manifest_chunk_metadata(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output_layout,
            output_name,
            file,
            output_location.as_ref(),
            script_link_plan,
        )?;

        Ok(ManifestFileRecord {
            path,
            file_type: self.build_manifest_file_type(file),
            loader: self.build_manifest_loader(file),
            chunk,
        }
        .into())
    }

    /// Build chunk metadata for one manifest-visible output file when it exists.
    fn build_manifest_chunk_metadata(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output_layout: &OutputLayout<'_>,
        output_name: TargetOutputName,
        file: &OutputFile,
        output_location: Option<&OutputLocation>,
        script_link_plan: Option<&ScriptLinkPlan>,
    ) -> LinkResult<Option<ManifestChunkMetadata>> {
        // html documents
        if matches!(file.content.file_type(), FileType::Html) {
            return Ok(Some(ManifestChunkMetadata {
                name: None,
                input: None,
                is_entry: output_name == TargetOutputName::Document,
                is_dynamic_entry: false,
                imports: Vec::new(),
                dynamic_imports: Vec::new(),
            }));
        }

        // non chunk files
        if !self.is_manifest_chunk_output(file) {
            return Ok(None);
        }

        // script plan
        let Some(script_link_plan) = script_link_plan else {
            return Ok(None);
        };

        // entry output
        if matches!(output_name, TargetOutputName::Entry) {
            return Ok(Some(self.build_entry_manifest_chunk_metadata(
                package_dir,
                target,
                script_link_plan,
            )));
        }

        // non module outputs
        if !matches!(output_name, TargetOutputName::Module) {
            return Ok(None);
        }

        // emitted module output
        let Some(output_location) = output_location else {
            return Ok(None);
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
            return Ok(None);
        };

        // module metadata
        Ok(Some(self.build_module_manifest_chunk_metadata(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output_layout,
            module_id,
            script_link_plan,
        )?))
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
        // reverse lookup
        for module_id in script_link_plan.modules() {
            let module_output_path = self.resolve_manifest_emitted_module_path(
                package_dir,
                root_dir,
                target,
                package_id,
                *module_id,
                script_link_plan,
            )?;

            let Some(module_output_path) = module_output_path else {
                continue;
            };

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
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<Vec<String>> {
        // artifact
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

        // source output
        let from_output = self.resolve_manifest_module_output_location(
            package_dir,
            root_dir,
            target,
            package_id,
            output_layout,
            module_id,
            script_link_plan,
        )?;

        // rewritten imports
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
                script_link_plan,
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
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<Vec<String>> {
        // artifact
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

        // source output
        let from_output = self.resolve_manifest_module_output_location(
            package_dir,
            root_dir,
            target,
            package_id,
            output_layout,
            module_id,
            script_link_plan,
        )?;

        // rewritten dynamic imports
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
                        script_link_plan,
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
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<String> {
        match dependency_target {
            // internal module edge
            ScriptDependencyTarget::Module { module, .. } => {
                let to_output = self.resolve_manifest_module_output_location(
                    package_dir,
                    root_dir,
                    target,
                    package_id,
                    output_layout,
                    *module,
                    script_link_plan,
                )?;

                Ok(output_layout.output_reference(
                    from_output,
                    &to_output,
                    OutputReferenceKind::Import,
                ))
            }

            // retained external
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

    /// Resolve one manifest-visible module output location for the current assembly mode.
    fn resolve_manifest_module_output_location(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        package_id: PackageId,
        output_layout: &OutputLayout<'_>,
        module_id: ModuleId,
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<OutputLocation> {
        // emitted path
        let output_path = match self.package_assembly(target) {
            PackageAssembly::Chunked => self
                .resolve_script_chunk_output_location(
                    module_id,
                    script_link_plan,
                    package_dir,
                    target,
                )
                .path()
                .to_path_buf(),
            PackageAssembly::PreserveModules => self.resolve_script_module_output_path(
                module_id,
                target,
                package_dir,
                root_dir,
                package_id,
            )?,
            PackageAssembly::SingleFile => {
                output_layout.script_entry_location().path().to_path_buf()
            }
        };

        Ok(output_layout.output_location(output_path))
    }

    /// Build one manifest index path when the target publishes one document.
    fn build_manifest_index_path(
        &self,
        output_layout: &OutputLayout<'_>,
        target: &Target,
        output: &destack_artifact::PackageOutput,
    ) -> Option<String> {
        // non html targets
        if target.emit != EmitFormat::Html {
            return None;
        }

        // document output
        output
            .outputs
            .get(&TargetOutputName::Document)
            .and_then(|files| files.first())
            .and_then(|file| self.file_output_location(output_layout, file))
            .map(|location| output_layout.manifest_path(&location))
    }

    /// Build entry metadata for one manifest chunk record.
    fn build_entry_manifest_chunk_metadata(
        &self,
        package_dir: &Path,
        target: &Target,
        script_link_plan: &ScriptLinkPlan,
    ) -> ManifestChunkMetadata {
        ManifestChunkMetadata {
            name: Some(target.name.clone()),
            input: script_link_plan
                .first_entry_module()
                .map(|entry_module| self.package_relative_module_path(package_dir, entry_module)),
            is_entry: true,
            is_dynamic_entry: false,
            imports: script_link_plan.external_targets().cloned().collect(),
            dynamic_imports: script_link_plan.dynamic_targets().cloned().collect(),
        }
    }

    /// Build module metadata for one manifest chunk record.
    fn build_module_manifest_chunk_metadata(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target_id: &TargetId,
        package_id: PackageId,
        target: &Target,
        output_layout: &OutputLayout<'_>,
        module_id: ModuleId,
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<ManifestChunkMetadata> {
        // chunk name
        let name = if self.package_assembly(target) == PackageAssembly::Chunked {
            Some(self.script_chunk_name(module_id, script_link_plan, package_dir))
        } else {
            None
        };

        // source module
        let input = Some(self.package_relative_module_path(package_dir, module_id));
        let is_entry = script_link_plan.entry_modules().contains(&module_id);
        let is_dynamic_entry = false;

        // rewritten imports
        let imports = self.build_manifest_imports_for_module(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output_layout,
            module_id,
            script_link_plan,
        )?;
        let dynamic_imports = self.build_manifest_dynamic_imports_for_module(
            package_dir,
            root_dir,
            target_id,
            package_id,
            target,
            output_layout,
            module_id,
            script_link_plan,
        )?;

        Ok(ManifestChunkMetadata {
            name,
            input,
            is_entry,
            is_dynamic_entry,
            imports,
            dynamic_imports,
        })
    }

    /// Resolve one emitted module path for reverse manifest lookup.
    fn resolve_manifest_emitted_module_path(
        &self,
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        package_id: PackageId,
        module_id: ModuleId,
        script_link_plan: &ScriptLinkPlan,
    ) -> LinkResult<Option<std::path::PathBuf>> {
        let output_path = match self.package_assembly(target) {
            PackageAssembly::Chunked => Some(
                self.resolve_script_chunk_output_location(
                    module_id,
                    script_link_plan,
                    package_dir,
                    target,
                )
                .path()
                .to_path_buf(),
            ),
            PackageAssembly::PreserveModules => Some(self.resolve_script_module_output_path(
                module_id,
                target,
                package_dir,
                root_dir,
                package_id,
            )?),
            PackageAssembly::SingleFile => None,
        };

        Ok(output_path)
    }
}
