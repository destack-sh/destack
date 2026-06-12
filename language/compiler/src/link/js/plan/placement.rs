use std::collections::HashSet;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use crate::{LinkError, LinkResult};
use destack_core::{StableHasher, stable_hash_bytes};
use destack_repository::{BundleFormat, BundleMode, Module, Target};
use destack_source::{FileContent, FileType, ModuleId};

use crate::link::{OutputFileNameValues, OutputLocation, TargetLocation, module_source_path};

use super::super::JsLinker;
use super::{OutputGraph, OutputId, OutputKind};

const DEFAULT_SCRIPT_ENTRY_FILE_NAME_TEMPLATE: &str = "[name].[ext]";
const DEFAULT_SCRIPT_SHARED_FILE_NAME_TEMPLATE: &str = "[name]-[hash].[ext]";

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
        file_type: FileType,
    ) -> Result<PathBuf, String> {
        let module_path = module_source_path(module)?;
        let extension = file_type
            .extension()
            .ok_or_else(|| format!("file type has no known extension: {file_type:?}"))?;

        Ok(target.resolve_out_file(package_dir, root_dir, &module_path, extension))
    }

    /// Resolve one preserve-modules JS output location for one module.
    pub(crate) fn module_output_location(
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        module: &Module,
        file_type: FileType,
    ) -> Result<OutputLocation, String> {
        let output_path =
            Self::module_output_path(package_dir, root_dir, target, module, file_type)?;

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
        if target.out_file.is_some() {
            return OutputLocation::new(output_layout.default_target_output_path("js"));
        }

        OutputLocation::new(output_layout.output_directory().join(output_name))
    }

    /// Build one emitted JS shared output location.
    pub(crate) fn shared_output_location(
        output_layout: &TargetLocation<'_>,
        target: &Target,
        name: &str,
        hash: Option<&str>,
    ) -> OutputLocation {
        let output_name = Self::render_shared_file_name(output_layout, target, name, hash);

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
            .bundle_output
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

    /// Render one configured JS shared file name.
    pub(crate) fn render_shared_file_name(
        output_layout: &TargetLocation<'_>,
        target: &Target,
        name: &str,
        hash: Option<&str>,
    ) -> String {
        let template = target
            .bundle_output
            .chunk_file_names
            .as_deref()
            .unwrap_or(DEFAULT_SCRIPT_SHARED_FILE_NAME_TEMPLATE);

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
        match target.bundle_output.format.unwrap_or(BundleFormat::Esm) {
            BundleFormat::Esm => "esm",
            BundleFormat::Iife => "iife",
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> JsLinker<'a> {
    /// Return one stable content hash for one source-backed module.
    fn source_hash(&self, module_id: ModuleId) -> LinkResult<String> {
        let module = self.module(module_id)?;
        let file = self.file(module.file_id)?;

        // hash the loaded content directly, regardless of file kind
        let hash = match file.content.payload() {
            FileContent::Text { content } => stable_hash_bytes(content.as_bytes()),
            FileContent::Binary { content } => stable_hash_bytes(content),
        };

        Ok(format!("{:08x}", hash as u32))
    }

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
            BundleMode::SingleFile => Ok(OutputLayout {
                output_names: vec![self.target_name().to_string()],
                output_locations: vec![OutputLayout::entry_output_location(
                    &output_layout,
                    self.target,
                    self.target_name(),
                    None,
                )],
            }),
            BundleMode::Chunked => self.build_chunked_output_layout(output_graph),
            BundleMode::PreserveModules => self.build_preserve_output_layout(output_graph),
        }
    }

    /// Build the chunked output layout over the current JS output graph.
    pub(crate) fn build_chunked_output_layout(
        &self,
        output_graph: &OutputGraph,
    ) -> LinkResult<OutputLayout> {
        let output_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());
        let mut used_names = HashSet::new();
        let mut used_output_paths = HashSet::new();
        let mut output_names = Vec::with_capacity(output_graph.outputs().len());
        let mut output_locations = Vec::with_capacity(output_graph.outputs().len());

        // derive unique names in stable output order
        for output in output_graph.outputs() {
            let mut output_name = if let Some(name) = output.manual_name() {
                name.to_string()
            } else {
                self.automatic_js_output_name(output)?
            };
            let base_name = output_name.clone();
            let mut duplicate_index = 2;

            while !used_names.insert(output_name.clone()) {
                output_name = format!("{base_name}-{duplicate_index}");
                duplicate_index += 1;
            }
            let output_hash = self.js_output_hash(output)?;

            let output_location = self.build_chunked_js_output_location(
                &output_layout,
                self.target,
                output.kind(),
                &output_name,
                Some(&output_hash),
            );

            // reject templates that collapse distinct outputs onto one path
            if !used_output_paths.insert(output_location.path().to_path_buf()) {
                return Err(LinkError::InvalidTarget {
                    anchor: self.package_id.into(),
                    package: self.package_id,
                    target: self.target_id.clone(),
                    message: format!(
                        "multiple JS outputs resolve to the same emitted path '{}'",
                        output_location.path().display()
                    ),
                });
            }

            output_names.push(output_name);
            output_locations.push(output_location);
        }

        Ok(OutputLayout {
            output_names,
            output_locations,
        })
    }

    /// Build the preserve-modules output layout over the current JS output graph.
    fn build_preserve_output_layout(&self, output_graph: &OutputGraph) -> LinkResult<OutputLayout> {
        let mut output_names = Vec::with_capacity(output_graph.outputs().len());
        let mut output_locations = Vec::with_capacity(output_graph.outputs().len());

        // each preserve-modules output maps to one module path
        for output in output_graph.outputs() {
            let module_id = output
                .facade_module()
                .or_else(|| output.modules().first().copied());
            let Some(module_id) = module_id else {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: "preserve-modules JS output had no facade or member modules"
                        .to_string(),
                });
            };
            let file_type = JsLinker::js_output_file_type(self)?;
            let output_location = OutputLayout::module_output_location(
                self.package_dir,
                self.root_dir,
                self.target,
                self.module(module_id)?.as_ref(),
                file_type,
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

    /// Build one emitted output location for one chunked output.
    fn build_chunked_js_output_location(
        &self,
        output_layout: &TargetLocation<'_>,
        target: &Target,
        kind: OutputKind,
        name: &str,
        hash: Option<&str>,
    ) -> OutputLocation {
        match kind {
            OutputKind::Entry => {
                OutputLayout::entry_output_location(output_layout, target, name, hash)
            }
            OutputKind::DynamicEntry | OutputKind::Shared => {
                OutputLayout::shared_output_location(output_layout, target, name, hash)
            }
        }
    }

    /// Build one stable emitted file-name hash for one JS output.
    fn js_output_hash(&self, output: &super::Output) -> LinkResult<String> {
        let mut hasher = StableHasher::new();

        // output shape
        output.kind().hash(&mut hasher);
        output.facade_module().hash(&mut hasher);
        output.manual_name().hash(&mut hasher);
        self.target.emit.hash(&mut hasher);

        // source-backed module content
        for module_id in output.modules() {
            module_id.hash(&mut hasher);

            let module = self.module(*module_id)?;
            let module = module.as_ref();

            if module.path.is_some() {
                let source_hash = self.source_hash(*module_id)?;

                source_hash.hash(&mut hasher);
            } else {
                module.uri.hash(&mut hasher);
            }
        }

        Ok(format!("{:08x}", hasher.finish_u64() as u32))
    }

    /// Build one default output name candidate for one automatic chunk.
    fn automatic_js_output_name(&self, output: &super::Output) -> LinkResult<String> {
        if output.kind() == OutputKind::Shared && output.facade_module().is_none() {
            return Ok("chunk".to_string());
        }

        let module_id = output
            .facade_module()
            .or_else(|| output.modules().first().copied())
            .unwrap_or_else(|| unreachable!("JS output should contain at least one module"));

        self.base_js_output_name(module_id, self.package_dir)
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
    use destack_source::{FileId, FileType, LanguageType, Loader, ModuleId, PackageId, Uri};

    use crate::link::{OutputLayout, TargetLocation};

    /// Render configured entry and shared output file names.
    #[test]
    fn test_renders_configured_js_output_file_names() {
        let mut target = Target::js();
        target.out_dir = Path::new("dist/bundle").to_path_buf();
        target.bundle_output.entry_file_names = Some("entries/[name]-entry.[ext]".to_string());
        target.bundle_output.chunk_file_names = Some("chunks/[name]-shared.[ext]".to_string());

        let layout = TargetLocation::new(Path::new("/workspace/pkg"), &target, "bundle");
        let entry_path = OutputLayout::entry_output_location(&layout, &target, "application", None);
        let shared_path =
            OutputLayout::shared_output_location(&layout, &target, "shared-value", None);

        assert_eq!(
            entry_path.path(),
            Path::new("/workspace/pkg/dist/bundle/entries/application-entry.js")
        );
        assert_eq!(
            shared_path.path(),
            Path::new("/workspace/pkg/dist/bundle/chunks/shared-value-shared.js")
        );
    }

    /// Render configured entry file name templates for linked JS entries.
    #[test]
    fn test_renders_entry_file_name_template_for_script_entry() {
        let mut target = Target::js();
        target.bundle_output.entry_file_names = Some("entries/[name]-bundle.[ext]".to_string());

        let layout = TargetLocation::new(Path::new("/workspace/pkg"), &target, "app");
        let entry_path = OutputLayout::entry_output_location(&layout, &target, "app", None);

        assert_eq!(
            entry_path.path(),
            Path::new("/workspace/pkg/dist/entries/app-bundle.js")
        );
    }

    /// Render configured shared file name templates for linked JS outputs.
    #[test]
    fn test_renders_shared_file_name_template_for_js_output() {
        let mut target = Target::js();
        target.bundle_output.chunk_file_names = Some("chunks/[name]-shared.[ext]".to_string());

        let layout = TargetLocation::new(Path::new("/workspace/pkg"), &target, "app");
        let shared_path =
            OutputLayout::shared_output_location(&layout, &target, "shared-value", None);

        assert_eq!(
            shared_path.path(),
            Path::new("/workspace/pkg/dist/chunks/shared-value-shared.js")
        );
    }

    /// Resolve preserve-modules JS output paths from module source paths.
    #[test]
    fn test_resolves_script_module_output_path() {
        let target = Target::js();
        let package_id = PackageId::from_path(Path::new("/workspace/pkg"));
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
            FileType::JavaScript,
        )
        .unwrap();

        assert_eq!(output_path, Path::new("/workspace/pkg/dist/util/math.js"));
    }
}
