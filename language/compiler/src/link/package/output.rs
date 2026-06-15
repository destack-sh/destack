use destack_repository::ProviderContext;
use std::path::Path;

use crate::{Compiler, CompilerResult};

use destack_artifact::{
    BuildManifest, OutputFile, PackageAssembly, PackageOutput, SourceMapArtifact, TargetOutputName,
};
use destack_repository::{BundleMode, RepositoryError, Target};
use destack_source::{Content, FileType, ModuleId, Uri};

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
    ) -> Result<(), RepositoryError> {
        let manifest_content = serde_json::to_string_pretty(&manifest)
            .unwrap_or_else(|_| serde_json::to_string(&manifest).unwrap_or_default());

        let output_layout = TargetLocation::new(package_dir, target, target_name);
        let manifest_path = output_layout.manifest_location();

        let file = self.intern_output_file(
            Uri::from_path(manifest_path.path()),
            FileType::Json,
            Self::text_output_content(manifest_content),
            None,
        )?;

        output
            .outputs
            .entry(TargetOutputName::Manifest)
            .or_default()
            .push(file);

        Ok(())
    }

    /// Intern one output file payload and return its artifact record.
    pub(crate) fn intern_output_file(
        &self,
        uri: Uri,
        file_type: FileType,
        content: Content,
        source: Option<Uri>,
    ) -> Result<OutputFile, RepositoryError> {
        let content = self.repository.intern_content(content)?;

        Ok(OutputFile::new(uri, file_type, content, source))
    }

    /// Build one normalized text output payload.
    pub(crate) fn text_output_content(mut content: String) -> Content {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }

        Content::Text { content }
    }

    /// Build one source map output payload.
    pub(crate) fn source_map_content(
        source_map: &SourceMapArtifact,
    ) -> Result<Content, serde_json::Error> {
        let content = serde_json::to_string(source_map)?;

        Ok(Self::text_output_content(content))
    }
}
