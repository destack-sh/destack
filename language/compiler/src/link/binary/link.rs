use destack_artifact::{
    ArtifactKey, BuildManifest, BuildManifestFile, ModuleOutput, OutputFile, PackageOutput,
    TargetOutputName,
};
use destack_repository::ProviderError;
use destack_source::{FileType, ModuleId};
use indexmap::IndexMap;

use crate::link::TargetLocation;
use crate::{Compiler, CompilerError, CompilerResult, LinkError, LinkResult};

use super::BinaryLinker;
use super::output::link_binary_output_files;

impl<'a> BinaryLinker<'a> {
    /// Link one discovered binary target.
    pub(crate) fn link_target(
        &self,
        discovered_modules: &[ModuleId],
    ) -> CompilerResult<PackageOutput> {
        let module_ids = self.require_module_outputs(discovered_modules)?;

        self.link(&module_ids).map_err(CompilerError::from)
    }

    /// Link one binary target from generated module outputs.
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
                self.target_name(),
                &mut output,
                manifest,
            )?;
        }

        Ok(output)
    }

    /// Require all generated binary outputs needed for this target.
    fn require_module_outputs(
        &self,
        discovered_modules: &[ModuleId],
    ) -> CompilerResult<Vec<ModuleId>> {
        let mut blocked = Vec::new();
        let mut required_modules = Vec::new();

        // require one generated output per discovered module
        for module_id in discovered_modules.iter().copied() {
            match self
                .artifacts
                .require(ArtifactKey::module_output(module_id, *self.target_id))
            {
                Ok(_) => {}
                Err(ProviderError::Blocked { keys }) => blocked.extend(keys),
                Err(error) => return Err(CompilerError::from(error)),
            }
            required_modules.push(module_id);
        }

        // yield while generated outputs are still pending
        if !blocked.is_empty() {
            return Err(CompilerError::Blocked { keys: blocked });
        }

        required_modules.sort_unstable();
        required_modules.dedup();

        Ok(required_modules)
    }

    /// Render final output files from generated binary outputs.
    fn render_files(&self, module_ids: &[ModuleId]) -> LinkResult<Vec<OutputFile>> {
        let mut files = Vec::new();

        // render each generated binary output into final target files
        for module_id in module_ids {
            let artifact = self
                .artifacts
                .module_output(*module_id, *self.target_id)
                .map_err(|error| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "missing module output for module {:?} target '{}': {error:?}",
                        module_id,
                        self.target_name()
                    ),
                })?;

            let ModuleOutput::Binary(binary) = artifact.as_ref() else {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "expected binary output for module {:?} target '{}'",
                        module_id,
                        self.target_name()
                    ),
                }
                .into());
            };

            let module = self
                .compiler
                .module(self.context.revision(), *module_id)
                .map_err(|error| Compiler::link_error(self.package_id, error))?;
            let binary_files = link_binary_output_files(
                module.as_ref(),
                binary,
                self.target,
                self.package_dir,
                self.root_dir,
            )
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "failed to render binary output for module {:?}: unsupported file type {:?}",
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
        let output_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
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

        BuildManifest { index: None, files }
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
