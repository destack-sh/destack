use crate::link::TargetLocation;
use crate::{LinkError, LinkResult};
use destack_artifact::{BuildManifest, BuildManifestFile, Bundle, BundleFile, BundleSection};
use destack_repository::JsOutputMode;

use super::JsLinker;
use super::plan::Plan;

impl<'a> JsLinker<'a> {
    /// Build one public build manifest for one JS target.
    pub(crate) fn build_js_manifest(
        &self,
        output: &Bundle,
        plan: &Plan,
    ) -> LinkResult<BuildManifest> {
        let target_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut files = Vec::new();
        for file in output.files() {
            let file = self.build_js_manifest_file(&target_layout, file, plan)?;

            files.push(file);
        }

        files.sort_by(|left, right| left.path.cmp(&right.path));

        Ok(BuildManifest { index: None, files })
    }

    /// Build one public build manifest file for one JS output.
    fn build_js_manifest_file(
        &self,
        target_layout: &TargetLocation<'_>,
        file: &BundleFile,
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
        let mut manifest = BuildManifestFile {
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
        };

        // non-script outputs use the common manifest fields
        if !matches!(file.section, BundleSection::Entry | BundleSection::Module) {
            return Ok(manifest);
        }

        let output_location = output_location
            .as_ref()
            .ok_or_else(|| LinkError::Internal {
                anchor: self.package_id.into(),
                package: self.package_id,
                message: format!("missing output location for JavaScript file '{}'", file.uri),
            })?;
        let output_id = plan
            .output_layout()
            .output_id_for_output_location(output_location)
            .ok_or_else(|| LinkError::Internal {
                anchor: self.package_id.into(),
                package: self.package_id,
                message: format!(
                    "missing output graph node for JavaScript file '{}'",
                    file.uri
                ),
            })?;
        let output = plan
            .output_graph()
            .output(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing output graph node for output id {}", output_id.0),
            })?;

        let output_location = plan
            .output_layout()
            .output_location(output_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("missing output placement for output id {}", output_id.0),
            })?;
        let mut imports = Vec::new();

        // resolve every planned output dependency
        for dependency_output_id in output.static_output_dependencies() {
            plan.output_graph()
                .output(*dependency_output_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: self.package_id.into(),
                    package: self.package_id,
                    message: format!(
                        "missing output graph node for dependency output id {}",
                        dependency_output_id.0
                    ),
                })?;
            let dependency_output_location = plan
                .output_layout()
                .output_location(*dependency_output_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: self.package_id.into(),
                    package: self.package_id,
                    message: format!(
                        "missing output placement for dependency output id {}",
                        dependency_output_id.0
                    ),
                })?;
            let reference =
                target_layout.output_reference(output_location, dependency_output_location);

            imports.push(reference);
        }
        imports.extend(output.external_imports().iter().cloned());

        manifest.name = if plan.output_graph().bundle_mode() == JsOutputMode::PreserveModules {
            None
        } else {
            plan.output_layout()
                .output_name(output_id)
                .map(ToString::to_string)
        };
        manifest.input = Some(
            self.compiler
                .package_relative_module_path(
                    self.package_dir,
                    output.facade_module(),
                    self.context,
                )
                .map_err(|error| self.link_error(error))?,
        );
        manifest.is_entry = Some(output.is_entry());
        manifest.is_dynamic_entry = Some(false);
        manifest.imports = imports;

        Ok(manifest)
    }
}
