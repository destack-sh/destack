use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, BuildManifest, BuildManifestFile, Bundle, BundleFile,
};
use destack_source::ModuleId;

use crate::link::TargetLocation;
use crate::{Compiler, CompilerError, CompilerResult, LinkError, LinkResult};

use super::NativeLinker;
use super::output::link_object_files;

impl<'a> NativeLinker<'a> {
    /// Declare the emitted outputs needed to link one native target.
    pub(crate) fn collect_modules(
        &self,
        discovered_modules: &[ModuleId],
        dependencies: &mut ArtifactDependencySet,
    ) {
        for module_id in discovered_modules {
            dependencies.require(ArtifactKey::object(*module_id, *self.target_id));
        }
    }

    /// Link one discovered native target.
    pub(crate) fn link_target(&self, discovered_modules: &[ModuleId]) -> CompilerResult<Bundle> {
        let mut module_ids = discovered_modules.to_vec();
        module_ids.sort_unstable();
        module_ids.dedup();

        self.link(&module_ids).map_err(CompilerError::from)
    }

    /// Link one native target from emitted objects.
    pub(crate) fn link(&self, module_ids: &[ModuleId]) -> LinkResult<Bundle> {
        // rendered files
        let files = self.render_files(module_ids)?;
        let mut output = self.build_bundle(files);

        // optional manifest
        if self.target.js.output.manifest {
            let manifest = self.build_manifest(&output);

            self.compiler
                .append_manifest_output(
                    self.package_dir,
                    self.target,
                    self.target_name(),
                    &mut output,
                    manifest,
                )
                .map_err(|error| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: error.to_string(),
                })?;
        }

        Ok(output)
    }

    /// Render final output files from emitted object assets.
    fn render_files(&self, module_ids: &[ModuleId]) -> LinkResult<Vec<BundleFile>> {
        let mut files = Vec::new();

        // render each emitted object into final target files
        for module_id in module_ids {
            let object = self
                .artifacts
                .object(*module_id, *self.target_id)
                .map_err(|error| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "missing object for module {:?} target '{}': {error:?}",
                        module_id,
                        self.target_name()
                    ),
                })?;

            let module = self
                .compiler
                .module(self.context.revision(), *module_id)
                .map_err(|error| Compiler::link_error(self.package_id, error))?;
            let native_files = link_object_files(
                self.compiler,
                module.as_ref(),
                object.as_ref(),
                self.target,
                self.package_dir,
                self.root_dir,
            )
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "failed to render object for module {:?}: unsupported file type {:?}",
                    module_id, error.file_type
                ),
            })?;

            files.extend(native_files);
        }

        Ok(files)
    }

    /// Build the packaged native output groups for this target.
    fn build_bundle(&self, files: Vec<BundleFile>) -> Bundle {
        Bundle::new(
            self.target.emit,
            Compiler::package_assembly(self.target.js.mode),
            files,
        )
    }

    /// Build the public build manifest for this native target.
    fn build_manifest(&self, output: &Bundle) -> BuildManifest {
        let output_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut files = output
            .files()
            .map(|file| {
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
            .collect::<Vec<_>>();

        // stable manifest order
        files.sort_by(|left, right| left.path.cmp(&right.path));

        BuildManifest { index: None, files }
    }
}
