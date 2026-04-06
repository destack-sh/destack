use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{LinkError, LinkResult};
use destack_artifact::EmitFormat;
use destack_source::{FileType, ModuleId};
use destack_workspace::{BundleMode, Module, Target};

use crate::link::{OutputLayout, OutputLocation, module_output_base_path};

use super::{ScriptLinker, ScriptOutputGraph, ScriptOutputId, ScriptOutputKind};

/// One output placement in one script output layout.
#[derive(Debug, Clone)]
pub(crate) struct ScriptOutputPlacement {
    /// The stable emitted output name.
    name: String,
    /// The resolved emitted output location.
    output_location: OutputLocation,
}

impl ScriptOutputPlacement {
    /// Return the stable emitted output name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Return the emitted output location.
    pub(crate) fn output_location(&self) -> &OutputLocation {
        &self.output_location
    }
}

/// One output layout for one script target.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScriptOutputLayout {
    /// The emitted output placements in stable output-id order.
    placements: Vec<ScriptOutputPlacement>,
}

impl ScriptOutputLayout {
    /// Return one output placement by its stable output id when it exists.
    pub(crate) fn placement(&self, output_id: ScriptOutputId) -> Option<&ScriptOutputPlacement> {
        self.placements.get(output_id.0)
    }

    /// Return one emitted output name by its stable output id when it exists.
    pub(crate) fn output_name(&self, output_id: ScriptOutputId) -> Option<&str> {
        self.placement(output_id).map(|placement| placement.name())
    }

    /// Return one emitted output location by its stable output id when it exists.
    pub(crate) fn output_location(&self, output_id: ScriptOutputId) -> Option<&OutputLocation> {
        self.placement(output_id)
            .map(|placement| placement.output_location())
    }

    /// Return one output id by its emitted output location when it exists.
    pub(crate) fn output_id_for_output_location(
        &self,
        output_location: &OutputLocation,
    ) -> Option<ScriptOutputId> {
        self.placements
            .iter()
            .position(|placement| placement.output_location() == output_location)
            .map(ScriptOutputId)
    }

    /// Resolve one preserve-modules script output path for one module.
    pub(crate) fn module_output_path(
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        module: &Module,
        file_type: FileType,
    ) -> Result<PathBuf, String> {
        let module_path = module_output_base_path(module);
        let extension = file_type
            .extension()
            .ok_or_else(|| format!("file type has no known extension: {file_type:?}"))?;

        Ok(target.resolve_out_file(package_dir, root_dir, &module_path, extension))
    }

    /// Resolve one preserve-modules script output location for one module.
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

    /// Build one emitted script entry output location.
    pub(crate) fn entry_output_location(
        output_layout: &OutputLayout<'_>,
        target: &Target,
        name: &str,
    ) -> OutputLocation {
        let output_name = Self::render_entry_file_name(output_layout, target, name);

        // HTML targets emit one runtime document plus one JS entry beside it
        if target.emit == EmitFormat::Html {
            return OutputLocation::new(
                output_layout
                    .document_location()
                    .path()
                    .with_file_name(output_name),
            );
        }

        // explicit out_file wins for single-file entry outputs
        if target.out_file.is_some() {
            return OutputLocation::new(output_layout.default_target_output_path("js"));
        }

        OutputLocation::new(output_layout.output_directory().join(output_name))
    }

    /// Build one emitted script shared output location.
    pub(crate) fn shared_output_location(
        output_layout: &OutputLayout<'_>,
        target: &Target,
        name: &str,
    ) -> OutputLocation {
        let output_name = Self::render_shared_file_name(output_layout, target, name);

        OutputLocation::new(output_layout.output_directory().join(output_name))
    }

    /// Render one configured script entry file name.
    pub(crate) fn render_entry_file_name(
        output_layout: &OutputLayout<'_>,
        target: &Target,
        name: &str,
    ) -> String {
        output_layout.render_output_file_name(
            target.bundle.output.entry_file_names.as_deref(),
            name,
            "js",
        )
    }

    /// Render one configured script shared file name.
    pub(crate) fn render_shared_file_name(
        output_layout: &OutputLayout<'_>,
        target: &Target,
        name: &str,
    ) -> String {
        output_layout.render_output_file_name(
            target.bundle.output.chunk_file_names.as_deref(),
            name,
            "js",
        )
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> ScriptLinker<'a> {
    /// Build the output layout over the current script output graph.
    pub(crate) fn build_script_output_layout(
        &self,
        output_graph: &ScriptOutputGraph,
    ) -> LinkResult<ScriptOutputLayout> {
        let output_layout = OutputLayout::new(self.package_dir, self.target);

        match output_graph.bundle_mode() {
            BundleMode::SingleFile => Ok(ScriptOutputLayout {
                placements: vec![ScriptOutputPlacement {
                    name: self.target.name.clone(),
                    output_location: ScriptOutputLayout::entry_output_location(
                        &output_layout,
                        self.target,
                        &self.target.name,
                    ),
                }],
            }),
            BundleMode::Chunked => self.build_chunked_script_output_layout(output_graph),
            BundleMode::PreserveModules => self.build_preserve_script_output_layout(output_graph),
        }
    }

    /// Build the chunked output layout over the current script output graph.
    pub(crate) fn build_chunked_script_output_layout(
        &self,
        output_graph: &ScriptOutputGraph,
    ) -> LinkResult<ScriptOutputLayout> {
        let output_layout = OutputLayout::new(self.package_dir, self.target);
        let mut used_names = HashSet::new();
        let mut placements = Vec::with_capacity(output_graph.outputs().len());

        // derive unique names in stable output order
        for output in output_graph.outputs() {
            let facade_module = output
                .facade_module()
                .or_else(|| output.modules().first().copied())
                .unwrap_or_else(|| panic!("output must contain at least one module"));
            let mut output_name = output
                .manual_name()
                .map(ToString::to_string)
                .unwrap_or_else(|| self.base_script_output_name(facade_module, self.package_dir));
            let base_name = output_name.clone();
            let mut duplicate_index = 2;

            while !used_names.insert(output_name.clone()) {
                output_name = format!("{base_name}-{duplicate_index}");
                duplicate_index += 1;
            }

            let output_location = self.build_chunked_script_output_location(
                &output_layout,
                self.target,
                output.kind(),
                &output_name,
            );

            placements.push(ScriptOutputPlacement {
                name: output_name,
                output_location,
            });
        }

        Ok(ScriptOutputLayout { placements })
    }

    /// Build the preserve-modules output layout over the current script output graph.
    fn build_preserve_script_output_layout(
        &self,
        output_graph: &ScriptOutputGraph,
    ) -> LinkResult<ScriptOutputLayout> {
        let mut placements = Vec::with_capacity(output_graph.outputs().len());

        // each preserve-modules output maps to one module path
        for output in output_graph.outputs() {
            let module_id = output
                .facade_module()
                .or_else(|| output.modules().first().copied())
                .unwrap_or_else(|| panic!("preserve output must contain at least one module"));
            let module = self.context.module(module_id);
            let file_type = self.script_output_file_type()?;
            let output_location = ScriptOutputLayout::module_output_location(
                self.package_dir,
                self.root_dir,
                self.target,
                module.as_ref(),
                file_type,
            )
            .map_err(|message| LinkError::Internal {
                package: self.package_id,
                message,
            })?;
            let output_name = self.base_script_output_name(module_id, Path::new(""));

            placements.push(ScriptOutputPlacement {
                name: output_name,
                output_location,
            });
        }

        Ok(ScriptOutputLayout { placements })
    }

    /// Build one emitted output location for one chunked output.
    fn build_chunked_script_output_location(
        &self,
        output_layout: &OutputLayout<'_>,
        target: &Target,
        kind: ScriptOutputKind,
        name: &str,
    ) -> OutputLocation {
        match kind {
            ScriptOutputKind::Entry => {
                ScriptOutputLayout::entry_output_location(output_layout, target, name)
            }
            ScriptOutputKind::DynamicEntry | ScriptOutputKind::Shared => {
                ScriptOutputLayout::shared_output_location(output_layout, target, name)
            }
        }
    }

    /// Build one default output name candidate for one module.
    fn base_script_output_name(&self, module_id: ModuleId, package_dir: &Path) -> String {
        let module_path =
            self.compiler
                .package_relative_module_path(package_dir, module_id, self.context);
        let module_path = Path::new(&module_path);
        let stem = module_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("output");

        self.sanitize_script_output_name(stem)
    }

    /// Sanitize one output name for output file templates.
    fn sanitize_script_output_name(&self, name: &str) -> String {
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

    /// Return the emitted file type for one script output node.
    fn script_output_file_type(&self) -> LinkResult<FileType> {
        match self.target.emit {
            EmitFormat::Js | EmitFormat::Html => Ok(FileType::JavaScript),
            EmitFormat::Ts => Ok(FileType::TypeScript),
            other => Err(LinkError::Internal {
                package: self.package_id,
                message: format!("unsupported script output file type: {other:?}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use destack_artifact::Loader;
    use destack_source::{FileId, FileType, LanguageType, ModuleId, PackageId, Uri};
    use destack_workspace::{Module, ModuleSource, Target};

    use crate::link::{OutputLayout, ScriptOutputLayout};

    /// Render configured entry and shared output file names.
    #[test]
    fn test_renders_configured_script_output_file_names() {
        let mut target = Target::js("bundle");
        target.out_dir = Path::new("dist/bundle").to_path_buf();
        target.bundle.output.entry_file_names = Some("entries/[name]-entry.[ext]".to_string());
        target.bundle.output.chunk_file_names = Some("chunks/[name]-shared.[ext]".to_string());

        let layout = OutputLayout::new(Path::new("/workspace/pkg"), &target);
        let entry_path = ScriptOutputLayout::entry_output_location(&layout, &target, "application");
        let shared_path =
            ScriptOutputLayout::shared_output_location(&layout, &target, "shared-value");

        assert_eq!(
            entry_path.path(),
            Path::new("/workspace/pkg/dist/bundle/entries/application-entry.js")
        );
        assert_eq!(
            shared_path.path(),
            Path::new("/workspace/pkg/dist/bundle/chunks/shared-value-shared.js")
        );
    }

    /// Render configured entry file name templates for linked script entries.
    #[test]
    fn test_renders_entry_file_name_template_for_script_entry() {
        let mut target = Target::js("app");
        target.bundle.output.entry_file_names = Some("entries/[name]-bundle.[ext]".to_string());

        let layout = OutputLayout::new(Path::new("/workspace/pkg"), &target);
        let entry_path = ScriptOutputLayout::entry_output_location(&layout, &target, "app");

        assert_eq!(
            entry_path.path(),
            Path::new("/workspace/pkg/dist/entries/app-bundle.js")
        );
    }

    /// Render configured shared file name templates for linked script outputs.
    #[test]
    fn test_renders_shared_file_name_template_for_script_output() {
        let mut target = Target::js("app");
        target.bundle.output.chunk_file_names = Some("chunks/[name]-shared.[ext]".to_string());

        let layout = OutputLayout::new(Path::new("/workspace/pkg"), &target);
        let shared_path =
            ScriptOutputLayout::shared_output_location(&layout, &target, "shared-value");

        assert_eq!(
            shared_path.path(),
            Path::new("/workspace/pkg/dist/chunks/shared-value-shared.js")
        );
    }

    /// Resolve preserve-modules script output paths from module source paths.
    #[test]
    fn test_resolves_script_module_output_path() {
        let target = Target::js("app");
        let package_id = PackageId::from_path(Path::new("/workspace/pkg"));
        let module = Module::blank(
            ModuleId::from_relative_path(package_id, Path::new("src/util/math.ds")),
            FileId::new(1),
            Uri::from_path("src/util/math.ds"),
            Some(PathBuf::from("src/util/math.ds")),
            package_id,
            LanguageType::Destack,
            Loader::Destack,
            ModuleSource::User,
        );

        let output_path = ScriptOutputLayout::module_output_path(
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
