use std::path::Path;

use crate::Compiler;
use crate::link::{OutputLayout, OutputLocation};
use destack_artifact::{
    BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader, OutputFile,
    PackageOutput, TargetOutputName,
};
use destack_source::FileType;
use destack_workspace::{BundleMode, Target};

use super::{ScriptOutputGraph, ScriptOutputId, ScriptOutputLayout, ScriptOutputNode};

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
    /// The exposed output name.
    name: Option<String>,
    /// The source module path for this output.
    input: Option<String>,
    /// Whether this output is an entry.
    is_entry: bool,
    /// Whether this output is a dynamic entry.
    is_dynamic_entry: bool,
    /// Static import references from this output.
    imports: Vec<String>,
    /// Dynamic import references from this output.
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

impl Compiler {
    /// Build one public build manifest for one script target.
    pub(crate) fn build_script_manifest(
        &self,
        package_dir: &Path,
        target: &Target,
        output: &PackageOutput,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> BuildManifest {
        let target_layout = OutputLayout::new(package_dir, target);
        let mut files = output
            .outputs
            .iter()
            .flat_map(|(output_name, files)| {
                files.iter().map(|file| {
                    self.build_script_manifest_file(
                        package_dir,
                        &target_layout,
                        *output_name,
                        file,
                        output_graph,
                        output_layout,
                    )
                })
            })
            .collect::<Vec<_>>();

        files.sort_by(|left, right| left.path.cmp(&right.path));

        BuildManifest {
            index: self.build_manifest_index_path(&target_layout, target, output),
            files,
        }
    }

    /// Build one public build manifest file for one script output.
    fn build_script_manifest_file(
        &self,
        package_dir: &Path,
        target_layout: &OutputLayout<'_>,
        output_name: TargetOutputName,
        file: &OutputFile,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> BuildManifestFile {
        let output_location = self.file_output_location(target_layout, file);
        let path = output_location
            .as_ref()
            .map(|output_location| target_layout.manifest_path(output_location))
            .unwrap_or_else(|| self.package_relative_uri_path(package_dir, &file.uri));
        let chunk = self.build_script_manifest_output_metadata(
            package_dir,
            target_layout,
            output_name,
            file.content.file_type(),
            output_location.as_ref(),
            output_graph,
            output_layout,
        );

        ManifestFileRecord {
            path,
            file_type: self.build_manifest_file_type(file),
            loader: self.build_manifest_loader(file),
            chunk,
        }
        .into()
    }

    /// Build one manifest metadata record for one script output when one exists.
    fn build_script_manifest_output_metadata(
        &self,
        package_dir: &Path,
        target_layout: &OutputLayout<'_>,
        output_name: TargetOutputName,
        file_type: FileType,
        output_location: Option<&OutputLocation>,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> Option<ManifestChunkMetadata> {
        if matches!(file_type, FileType::Html) {
            return Some(ManifestChunkMetadata {
                name: None,
                input: None,
                is_entry: output_name == TargetOutputName::Document,
                is_dynamic_entry: false,
                imports: Vec::new(),
                dynamic_imports: Vec::new(),
            });
        }

        if !matches!(file_type, FileType::JavaScript | FileType::TypeScript) {
            return None;
        }

        let output_location = output_location?;
        let output_id = output_layout.output_id_for_output_location(output_location)?;
        let output = output_graph
            .output(output_id)
            .unwrap_or_else(|| panic!("missing output graph node for output id {}", output_id.0));

        Some(self.build_script_manifest_node_metadata(
            package_dir,
            target_layout,
            output_id,
            output,
            output_graph,
            output_layout,
        ))
    }

    /// Build one manifest metadata record for one output node.
    fn build_script_manifest_node_metadata(
        &self,
        package_dir: &Path,
        target_layout: &OutputLayout<'_>,
        output_id: ScriptOutputId,
        output: &ScriptOutputNode,
        output_graph: &ScriptOutputGraph,
        output_layout: &ScriptOutputLayout,
    ) -> ManifestChunkMetadata {
        let output_location = output_layout
            .output_location(output_id)
            .unwrap_or_else(|| panic!("missing output placement for output id {}", output_id.0));
        let mut imports = output
            .static_output_dependencies()
            .iter()
            .filter_map(|dependency_output_id| {
                let _ = output_graph.output(*dependency_output_id)?;
                let dependency_output_location =
                    output_layout.output_location(*dependency_output_id)?;

                Some(target_layout.output_reference(output_location, dependency_output_location))
            })
            .collect::<Vec<_>>();
        let mut dynamic_imports = output
            .dynamic_output_dependencies()
            .iter()
            .filter_map(|dependency_output_id| {
                let _ = output_graph.output(*dependency_output_id)?;
                let dependency_output_location =
                    output_layout.output_location(*dependency_output_id)?;

                Some(target_layout.output_reference(output_location, dependency_output_location))
            })
            .collect::<Vec<_>>();

        imports.extend(output.external_imports().iter().cloned());
        dynamic_imports.extend(output.external_dynamic_imports().iter().cloned());

        ManifestChunkMetadata {
            name: if output_graph.bundle_mode() == BundleMode::PreserveModules {
                None
            } else {
                output_layout
                    .output_name(output_id)
                    .map(ToString::to_string)
            },
            input: output
                .facade_module()
                .map(|module_id| self.package_relative_module_path(package_dir, module_id)),
            is_entry: output.is_entry(),
            is_dynamic_entry: output.is_dynamic_entry(),
            imports,
            dynamic_imports,
        }
    }
}
