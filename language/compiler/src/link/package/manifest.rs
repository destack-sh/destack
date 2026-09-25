use crate::Compiler;
use tspp_artifact::{BuildManifestFileType, BuildManifestLoader, BundleFile, BundleSection};
use tspp_source::FileType;

use super::layout::{OutputLocation, TargetLocation};

impl Compiler {
    /// Return one output location for one emitted file when its URI is path based.
    pub(crate) fn file_output_location(
        &self,
        output_layout: &TargetLocation<'_>,
        file: &BundleFile,
    ) -> Option<OutputLocation> {
        file.uri
            .to_path_buf()
            .map(|path| output_layout.output_location(path))
    }

    /// Return the manifest file type for one emitted file.
    pub(crate) fn build_manifest_file_type(&self, file: &BundleFile) -> BuildManifestFileType {
        match file.section {
            BundleSection::Entry | BundleSection::Module => BuildManifestFileType::Chunk,
            _ if matches!(file.file_type, FileType::Object | FileType::Wasm) => {
                BuildManifestFileType::Binary
            }
            _ => BuildManifestFileType::Asset,
        }
    }

    /// Return the manifest loader string for one emitted file.
    pub(crate) fn build_manifest_loader(&self, file: &BundleFile) -> BuildManifestLoader {
        match file.file_type {
            FileType::JavaScript => BuildManifestLoader::Js,
            FileType::Css => BuildManifestLoader::Css,
            FileType::SourceMap => BuildManifestLoader::Map,
            FileType::Json => BuildManifestLoader::Json,
            FileType::Wasm => BuildManifestLoader::Wasm,
            FileType::Object => BuildManifestLoader::Object,
            _ => BuildManifestLoader::Asset,
        }
    }
}
