use std::path::Path;
use tspp_repository::ProviderContext;

use crate::{Compiler, CompilerResult};

use tspp_artifact::{BuildManifest, Bundle, BundleFile, BundleMode, BundleSection, SourceMap};
use tspp_repository::{JsOutputMode, RepositoryError, Target};
use tspp_source::{FileType, ModuleId, Uri};

use super::layout::TargetLocation;

impl Compiler {
    /// Lower one bundle mode to the published package assembly shape.
    pub(crate) fn package_assembly(bundle_mode: JsOutputMode) -> BundleMode {
        match bundle_mode {
            JsOutputMode::PreserveModules => BundleMode::PreserveModules,
            JsOutputMode::SingleFile => BundleMode::SingleFile,
            JsOutputMode::Chunked => BundleMode::Chunked,
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

    /// Append one manifest sidecar to one bundle.
    pub(crate) fn append_manifest_output(
        &self,
        package_dir: &Path,
        target: &Target,
        target_name: &str,
        output: &mut Bundle,
        manifest: BuildManifest,
    ) -> Result<(), RepositoryError> {
        let manifest =
            manifest
                .to_json_value()
                .map_err(|error| RepositoryError::InvalidArtifact {
                    message: format!("failed to build manifest JSON: {error}"),
                })?;
        let manifest_text = serde_json::to_string_pretty(&manifest).map_err(|error| {
            RepositoryError::InvalidArtifact {
                message: format!("failed to serialize manifest JSON: {error}"),
            }
        })?;

        let output_layout = TargetLocation::new(package_dir, target, target_name);
        let manifest_path = output_layout.manifest_location();

        let bytes = Self::encode_output_text(manifest_text);
        let file = self.put_output_file(
            BundleSection::Manifest,
            Uri::from_path(manifest_path.path()),
            FileType::Json,
            &bytes,
            None,
        )?;

        output.files.push(file);

        Ok(())
    }

    /// Store exact bytes as one emitted output File.
    pub(crate) fn put_output_file(
        &self,
        section: BundleSection,
        uri: Uri,
        file_type: FileType,
        bytes: &[u8],
        source: Option<Uri>,
    ) -> Result<BundleFile, RepositoryError> {
        let blob = self.repository.retain_blob(bytes)?;

        Ok(BundleFile::new(section, uri, file_type, blob, source))
    }

    /// Encode one normalized text output.
    pub(crate) fn encode_output_text(mut text: String) -> Vec<u8> {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }

        text.into_bytes()
    }

    /// Encode one source map output.
    pub(crate) fn encode_source_map(source_map: &SourceMap) -> Result<Vec<u8>, serde_json::Error> {
        let source_map = source_map.to_json_value()?;
        let text = serde_json::to_string(&source_map)?;

        Ok(Self::encode_output_text(text))
    }
}
