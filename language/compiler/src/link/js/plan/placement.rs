use std::path::{Path, PathBuf};

use crate::{LinkError, LinkResult};
use destack_repository::{JsOutputFormat, JsOutputMode, Module, Target};
use destack_source::ModuleId;

use crate::link::{OutputFileNameValues, OutputLocation, TargetLocation, module_source_path};

use super::super::JsLinker;
use super::{OutputGraph, OutputId};

const DEFAULT_SCRIPT_ENTRY_FILE_NAME_TEMPLATE: &str = "[name].[ext]";

/// One output layout for one JS target.
#[derive(Debug, Clone, Default)]
pub(crate) struct OutputLayout {
    /// The emitted output names in stable output-id order.
    output_names: Vec<String>,
    /// The emitted output locations in stable output-id order.
    output_locations: Vec<OutputLocation>,
}

impl OutputLayout {
    /// Return one emitted output name by its stable output id when it exists.
    pub(crate) fn output_name(&self, output_id: OutputId) -> Option<&str> {
        self.output_names.get(output_id.0).map(String::as_str)
    }

    /// Return one emitted output location by its stable output id when it exists.
    pub(crate) fn output_location(&self, output_id: OutputId) -> Option<&OutputLocation> {
        self.output_locations.get(output_id.0)
    }

    /// Return one output id by its emitted output location when it exists.
    pub(crate) fn output_id_for_output_location(
        &self,
        output_location: &OutputLocation,
    ) -> Option<OutputId> {
        self.output_locations
            .iter()
            .position(|location| location == output_location)
            .map(OutputId)
    }

    /// Resolve one preserve-modules JS output path for one module.
    pub(crate) fn module_output_path(
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        module: &Module,
        extension: &str,
    ) -> Result<PathBuf, String> {
        let module_path = module_source_path(module)?;

        Ok(target.resolve_output_file(package_dir, root_dir, &module_path, extension))
    }

    /// Resolve one preserve-modules JS output location for one module.
    pub(crate) fn module_output_location(
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        module: &Module,
    ) -> Result<OutputLocation, String> {
        let output_path = Self::module_output_path(package_dir, root_dir, target, module, "js")?;

        Ok(OutputLocation::new(output_path))
    }

    /// Build one emitted JS entry output location.
    pub(crate) fn entry_output_location(
        output_layout: &TargetLocation<'_>,
        target: &Target,
        name: &str,
        hash: Option<&str>,
    ) -> OutputLocation {
        let output_name = Self::render_entry_file_name(output_layout, target, name, hash);

        // explicit out_file wins for single-file entry outputs
        if target.destination.file.is_some() {
            return OutputLocation::new(output_layout.default_target_output_path("js"));
        }

        OutputLocation::new(output_layout.output_directory().join(output_name))
    }

    /// Render one configured JS entry file name.
    pub(crate) fn render_entry_file_name(
        output_layout: &TargetLocation<'_>,
        target: &Target,
        name: &str,
        hash: Option<&str>,
    ) -> String {
        let template = target
            .js
            .output
            .entry_file_names
            .as_deref()
            .unwrap_or(DEFAULT_SCRIPT_ENTRY_FILE_NAME_TEMPLATE);

        output_layout.render_output_file_name_with_values(
            Some(template),
            OutputFileNameValues {
                directory: None,
                name,
                hash,
                format: Some(Self::js_output_format_name(target)),
                extension: "js",
            },
        )
    }

    /// Return the naming token for the configured JS output format.
    fn js_output_format_name(target: &Target) -> &'static str {
        match target.js.output.format.unwrap_or(JsOutputFormat::Esm) {
            JsOutputFormat::Esm => "esm",
            JsOutputFormat::Iife => "iife",
        }
    }
}

impl<'a> JsLinker<'a> {
    /// Build the output layout over the current JS output graph.
    pub(crate) fn build_output_layout(
        &self,
        output_graph: &OutputGraph,
    ) -> LinkResult<OutputLayout> {
        if output_graph.outputs().is_empty() {
            return Ok(OutputLayout::default());
        }

        let output_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());

        match output_graph.bundle_mode() {
            JsOutputMode::SingleFile => Ok(OutputLayout {
                output_names: vec![self.target_name().to_string()],
                output_locations: vec![OutputLayout::entry_output_location(
                    &output_layout,
                    self.target,
                    self.target_name(),
                    None,
                )],
            }),
            JsOutputMode::PreserveModules => self.build_preserve_output_layout(output_graph),
        }
    }

    /// Build the preserve-modules output layout over the current JS output graph.
    fn build_preserve_output_layout(&self, output_graph: &OutputGraph) -> LinkResult<OutputLayout> {
        let mut output_names = Vec::with_capacity(output_graph.outputs().len());
        let mut output_locations = Vec::with_capacity(output_graph.outputs().len());

        // each preserve-modules output maps to one module path
        for output in output_graph.outputs() {
            let module_id = output.facade_module();
            let output_location = OutputLayout::module_output_location(
                self.package_dir,
                self.root_dir,
                self.target,
                self.module(module_id)?.as_ref(),
            )
            .map_err(|message| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message,
            })?;
            let output_name = self.base_js_output_name(module_id, Path::new(""))?;

            output_names.push(output_name);
            output_locations.push(output_location);
        }

        Ok(OutputLayout {
            output_names,
            output_locations,
        })
    }

    /// Build one default output name candidate for one module.
    fn base_js_output_name(&self, module_id: ModuleId, package_dir: &Path) -> LinkResult<String> {
        let module_path = self
            .compiler
            .package_relative_module_path(package_dir, module_id, self.context)
            .map_err(|error| self.link_error(error))?;
        let module_path = Path::new(&module_path);
        let stem = module_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("output");

        Ok(self.sanitize_js_output_name(stem))
    }

    /// Sanitize one output name for output file templates.
    fn sanitize_js_output_name(&self, name: &str) -> String {
        let mut sanitized = String::new();
        let mut previous_was_dash = false;

        for character in name.chars() {
            if character.is_ascii_alphanumeric() || character == '_' {
                sanitized.push(character);
                previous_was_dash = false;
                continue;
            }

            if !previous_was_dash {
                sanitized.push('-');
                previous_was_dash = true;
            }
        }

        let sanitized = sanitized.trim_matches('-').to_string();

        if sanitized.is_empty() {
            return "output".to_string();
        }

        sanitized
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use destack_repository::{Module, Target};
    use destack_source::{FileId, LanguageType, Loader, ModuleId, PackageId, Uri};

    use crate::link::{OutputLayout, TargetLocation};

    /// Render configured entry file name templates for linked JS entries.
    #[test]
    fn test_renders_entry_file_name_template_for_script_entry() {
        let mut target = Target::js();
        target.js.output.entry_file_names = Some("entries/[name]-bundle.[ext]".to_string());

        let layout = TargetLocation::new(Path::new("/workspace/pkg"), &target, "app");
        let entry_path = OutputLayout::entry_output_location(&layout, &target, "app", None);

        assert_eq!(
            entry_path.path(),
            Path::new("/workspace/pkg/dist/entries/app-bundle.js")
        );
    }

    /// Resolve preserve-modules JS output paths from module source paths.
    #[test]
    fn test_resolves_script_module_output_path() {
        let target = Target::js();
        let package_id = PackageId::from_path(Path::new("pkg"));
        let module = Module::blank(
            ModuleId::from_relative_path(package_id, Path::new("src/util/math.ds")),
            FileId::from_logical_str("src/util/math.ds"),
            Uri::from_path("src/util/math.ds"),
            Some(PathBuf::from("src/util/math.ds")),
            package_id,
            Some(LanguageType::Destack),
            Loader::Destack,
        );

        let output_path = OutputLayout::module_output_path(
            Path::new("/workspace/pkg"),
            Some(Path::new("/workspace/pkg/src")),
            &target,
            &module,
            "js",
        )
        .unwrap();

        assert_eq!(output_path, Path::new("/workspace/pkg/dist/util/math.js"));
    }
}
