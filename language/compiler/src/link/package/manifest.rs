use crate::Compiler;
use destack_artifact::{BuildManifestFileType, BuildManifestLoader, OutputFile};
use destack_source::FileType;

use super::layout::{OutputLocation, TargetLocation};

impl Compiler {
    /// Return one output location for one emitted file when its URI is path based.
    pub(crate) fn file_output_location(
        &self,
        output_layout: &TargetLocation<'_>,
        file: &OutputFile,
    ) -> Option<OutputLocation> {
        file.uri
            .to_path_buf()
            .map(|path| output_layout.output_location(path))
    }

    /// Return the manifest file type for one emitted file.
    pub(crate) fn build_manifest_file_type(&self, file: &OutputFile) -> BuildManifestFileType {
        match file.content.file_type() {
            FileType::JavaScript | FileType::TypeScript => BuildManifestFileType::Chunk,
            FileType::Object | FileType::Wasm => BuildManifestFileType::Binary,
            _ => BuildManifestFileType::Asset,
        }
    }

    /// Return the manifest loader string for one emitted file.
    pub(crate) fn build_manifest_loader(&self, file: &OutputFile) -> BuildManifestLoader {
        match file.content.file_type() {
            FileType::JavaScript => BuildManifestLoader::Js,
            FileType::TypeScript => BuildManifestLoader::Ts,
            FileType::SourceMap => BuildManifestLoader::Map,
            FileType::Json => BuildManifestLoader::Json,
            FileType::TypeScriptDeclaration => BuildManifestLoader::Dts,
            FileType::Wasm => BuildManifestLoader::Wasm,
            FileType::Object => BuildManifestLoader::Object,
            _ => BuildManifestLoader::Asset,
        }
    }
}
