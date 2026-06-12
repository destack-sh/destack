use crate::link::{OutputLocation, TargetLocation};
use crate::{LinkError, LinkResult};
use destack_artifact::{
    BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader, OutputFile,
    PackageOutput,
};
use destack_repository::BundleMode;
use destack_source::FileType;

use super::JsLinker;
use super::plan::{Output, OutputId, Plan};

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
    /// Associated stylesheet references from this output.
    stylesheets: Vec<String>,
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
            stylesheets: chunk
                .as_ref()
                .map(|chunk| chunk.stylesheets.clone())
                .unwrap_or_default(),
        }
    }
}

impl<'a> JsLinker<'a> {
    /// Build one public build manifest for one JS target.
    pub(crate) fn build_js_manifest(
        &self,
        output: &PackageOutput,
        plan: &Plan,
    ) -> LinkResult<BuildManifest> {
        let target_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut files = Vec::new();
        for output_files in output.outputs.values() {
            for file in output_files {
                let file = self.build_js_manifest_file(&target_layout, file, plan)?;

                files.push(file);
            }
        }

        files.sort_by(|left, right| left.path.cmp(&right.path));

        Ok(BuildManifest { index: None, files })
    }

    /// Build one public build manifest file for one JS output.
    fn build_js_manifest_file(
        &self,
        target_layout: &TargetLocation<'_>,
        file: &OutputFile,
        plan: &Plan,
    ) -> LinkResult<BuildManifestFile> {
        let output_location = self.compiler.file_output_location(target_layout, file);
        let path = output_location
            .as_ref()
            .map(|output_location| target_layout.manifest_path(output_location))
            .unwrap_or_else(|| {
                self.compiler
                    .package_relative_uri_path(self.package_dir, &file.uri)
            });
        let chunk = self.build_js_manifest_output_metadata(
            target_layout,
            file.content.file_type(),
            output_location.as_ref(),
            plan,
        )?;

        Ok(ManifestFileRecord {
            path,
            file_type: self.compiler.build_manifest_file_type(file),
            loader: self.compiler.build_manifest_loader(file),
            chunk,
        }
        .into())
    }

    /// Build one manifest metadata record for one JS output when one exists.
    fn build_js_manifest_output_metadata(
        &self,
        target_layout: &TargetLocation<'_>,
        file_type: FileType,
        output_location: Option<&OutputLocation>,
        plan: &Plan,
    ) -> LinkResult<Option<ManifestChunkMetadata>> {
        if !matches!(file_type, FileType::JavaScript | FileType::TypeScript) {
            return Ok(None);
        }

        let Some(output_location) = output_location else {
            return Ok(None);
        };
        let Some(output_id) = plan
            .output_layout()
            .output_id_for_output_location(output_location)
        else {
            return Ok(None);
        };
        let output = plan
            .output_graph()
            .output(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing output graph node for output id {}", output_id.0),
            })?;

        Ok(Some(self.build_js_manifest_node_metadata(
            target_layout,
            output_id,
            output,
            plan,
        )?))
    }

    /// Build one manifest metadata record for one output node.
    fn build_js_manifest_node_metadata(
        &self,
        target_layout: &TargetLocation<'_>,
        output_id: OutputId,
        output: &Output,
        plan: &Plan,
    ) -> LinkResult<ManifestChunkMetadata> {
        let output_location = plan
            .output_layout()
            .output_location(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing output placement for output id {}", output_id.0),
            })?;
        let mut imports = output
            .static_output_dependencies()
            .iter()
            .filter_map(|dependency_output_id| {
                let _ = plan.output_graph().output(*dependency_output_id)?;
                let dependency_output_location = plan
                    .output_layout()
                    .output_location(*dependency_output_id)?;

                Some(target_layout.output_reference(output_location, dependency_output_location))
            })
            .collect::<Vec<_>>();
        let mut dynamic_imports = output
            .dynamic_output_dependencies()
            .iter()
            .filter_map(|dependency_output_id| {
                let _ = plan.output_graph().output(*dependency_output_id)?;
                let dependency_output_location = plan
                    .output_layout()
                    .output_location(*dependency_output_id)?;

                Some(target_layout.output_reference(output_location, dependency_output_location))
            })
            .collect::<Vec<_>>();
        imports.extend(output.external_imports().iter().cloned());
        dynamic_imports.extend(output.external_dynamic_imports().iter().cloned());

        Ok(ManifestChunkMetadata {
            name: if plan.output_graph().bundle_mode() == BundleMode::PreserveModules {
                None
            } else {
                plan.output_layout()
                    .output_name(output_id)
                    .map(ToString::to_string)
            },
            input: output
                .facade_module()
                .map(|module_id| {
                    self.compiler
                        .package_relative_module_path(self.package_dir, module_id, self.context)
                        .map_err(|error| self.link_error(error))
                })
                .transpose()?,
            is_entry: output.is_entry(),
            is_dynamic_entry: output.is_dynamic_entry(),
            imports,
            dynamic_imports,
            stylesheets: Vec::new(),
        })
    }
}
