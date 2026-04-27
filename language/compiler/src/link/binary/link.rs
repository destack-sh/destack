use crate::{Compiler, LinkError, LinkResult, RequirementCollector};

use destack_artifact::{
    BuildManifest, BuildManifestFile, ModuleOutput, OutputFile, PackageOutput, TargetOutputName,
};
use destack_source::{FileType, ModuleId};
use indexmap::IndexMap;

use crate::link::TargetLocation;

use super::BinaryLinker;
use super::output::link_binary_artifact_files;

impl<'a> BinaryLinker<'a> {
    /// Link one discovered binary target.
    pub(crate) fn link_target(&self, discovered_modules: &[ModuleId]) -> LinkResult<PackageOutput> {
        let module_ids = self.require_module_outputs(discovered_modules)?;

        self.link(&module_ids)
    }

    /// Link one binary target from generated module artifacts.
    pub(crate) fn link(&self, module_ids: &[ModuleId]) -> LinkResult<PackageOutput> {
        // rendered files
        let files = self.render_files(module_ids)?;
        let mut output = self.build_package_output(files);

        // optional manifest
        if self.target.bundle_output.manifest {
            let manifest = self.build_manifest(&output);

            self.compiler.append_manifest_output(
                self.package_dir,
                self.target,
                &mut output,
                manifest,
            )?;
        }

        Ok(output)
    }

    /// Require all generated binary artifacts needed for this target.
    fn require_module_outputs(&self, discovered_modules: &[ModuleId]) -> LinkResult<Vec<ModuleId>> {
        let mut collector = RequirementCollector::new();
        let mut required_modules = Vec::new();

        // require one generated artifact per discovered module
        for module_id in discovered_modules.iter().copied() {
            let profile_id = self
                .context
                .profile_id_for_target(module_id, self.target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: self.package_id,
                    message: format!(
                        "profile not found for target '{}'",
                        self.compiler
                            .target_name_for_revision(self.context.revision(), self.target_id)
                    ),
                })?;
            let result = self.compiler.require_module_output(
                self.context.revision(),
                module_id,
                profile_id,
                self.target_id,
            );
            collector.try_collect(result);
            required_modules.push(module_id);
        }

        // yield while generated artifacts are still pending
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(LinkError::Yield { requirement });
        }

        required_modules.sort_unstable();
        required_modules.dedup();

        Ok(required_modules)
    }

    /// Render final output files from generated binary artifacts.
    fn render_files(&self, module_ids: &[ModuleId]) -> LinkResult<Vec<OutputFile>> {
        let mut files = Vec::new();

        // render each generated binary artifact into final target files
        for module_id in module_ids {
            let artifact = self
                .compiler
                .module_output(*module_id, self.target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: self.package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id,
                        self.compiler
                            .target_name_for_revision(self.context.revision(), self.target_id)
                    ),
                })?;

            let ModuleOutput::Binary(binary) = artifact.as_ref() else {
                return Err(LinkError::Internal {
                    package: self.package_id,
                    message: format!(
                        "expected binary artifact for module {:?} target '{}'",
                        module_id,
                        self.compiler
                            .target_name_for_revision(self.context.revision(), self.target_id)
                    ),
                });
            };

            let module = self.context.module(*module_id);
            let binary_files = link_binary_artifact_files(
                module.as_ref(),
                binary,
                self.target,
                self.package_dir,
                self.root_dir,
            )
            .map_err(|error| LinkError::Internal {
                package: self.package_id,
                message: format!(
                    "failed to render binary artifact for module {:?}: unsupported file type {:?}",
                    module_id, error.file_type
                ),
            })?;

            files.extend(binary_files);
        }

        Ok(files)
    }

    /// Build the packaged binary output groups for this target.
    fn build_package_output(&self, files: Vec<OutputFile>) -> PackageOutput {
        let mut outputs = IndexMap::new();

        // group linked binary files by their emitted output role
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

    /// Build the public build manifest for this binary target.
    fn build_manifest(&self, output: &PackageOutput) -> BuildManifest {
        let output_layout = TargetLocation::new(self.package_dir, self.target);
        let mut files = output
            .outputs
            .values()
            .flat_map(|files| {
                files.iter().map(|file| {
                    let path = self
                        .compiler
                        .file_output_location(&output_layout, file)
                        .map(|output_location| output_layout.manifest_path(&output_location))
                        .unwrap_or_else(|| {
                            self.compiler
                                .package_relative_uri_path(self.package_dir, &file.uri)
                        });

                    BuildManifestFile {
                        path,
                        r#type: self.compiler.build_manifest_file_type(file),
                        loader: self.compiler.build_manifest_loader(file),
                        name: None,
                        input: None,
                        is_entry: None,
                        is_dynamic_entry: None,
                        imports: Vec::new(),
                        dynamic_imports: Vec::new(),
                        stylesheets: Vec::new(),
                    }
                })
            })
            .collect::<Vec<_>>();

        // stable manifest order
        files.sort_by(|left, right| left.path.cmp(&right.path));

        BuildManifest {
            index: self
                .compiler
                .build_manifest_index_path(&output_layout, self.target, output),
            files,
        }
    }

    /// Return the grouped output name for one emitted binary file.
    fn target_output_name_for_file(&self, file_type: FileType) -> TargetOutputName {
        match file_type {
            FileType::SourceMap => TargetOutputName::Maps,
            FileType::Object | FileType::Wasm => TargetOutputName::Binary,
            _ => TargetOutputName::Assets,
        }
    }
}
