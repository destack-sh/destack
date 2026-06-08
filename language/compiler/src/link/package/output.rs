use destack_repository::ProviderContext;
use std::path::Path;

use crate::{Compiler, CompilerResult, LinkResult};

use destack_artifact::{
    BuildManifest, OutputContent, OutputFile, PackageAssembly, PackageOutput, TargetOutputName,
};
use destack_repository::{BundleMode, Target};
use destack_source::{FileType, ModuleId, Uri};

use super::layout::TargetLocation;

impl Compiler {
    /// Lower one bundle mode to the published package assembly shape.
    pub(crate) fn package_assembly(bundle_mode: BundleMode) -> PackageAssembly {
        match bundle_mode {
            BundleMode::PreserveModules => PackageAssembly::PreserveModules,
            BundleMode::SingleFile => PackageAssembly::SingleFile,
            BundleMode::Chunked => PackageAssembly::Chunked,
        }
    }

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
        context: &dyn ProviderContext,
    ) -> CompilerResult<String> {
        let module = self.module(context.revision(), module_id)?;

        if let Some(path) = &module.path {
            return Ok(self.package_relative_path(package_dir, path));
        }

        Ok(self.package_relative_uri_path(package_dir, &module.uri))
    }

    /// Append one manifest sidecar to one package output.
    pub(crate) fn append_manifest_output(
        &self,
        package_dir: &Path,
        target: &Target,
        target_name: &str,
        output: &mut PackageOutput,
        manifest: BuildManifest,
    ) -> LinkResult<()> {
        let manifest_content = serde_json::to_string_pretty(&manifest)
            .unwrap_or_else(|_| serde_json::to_string(&manifest).unwrap_or_default());

        let output_layout = TargetLocation::new(package_dir, target, target_name);
        let manifest_path = output_layout.manifest_location();

        output
            .outputs
            .entry(TargetOutputName::Manifest)
            .or_default()
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
