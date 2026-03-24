use std::path::{Component, Path, PathBuf};

use destack_workspace::Target;

/// One resolved emitted output location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLocation {
    /// The absolute output path on disk.
    absolute_path: PathBuf,
}

impl OutputLocation {
    /// Create one output location from one absolute output path.
    pub(crate) fn new(absolute_path: PathBuf) -> Self {
        Self { absolute_path }
    }

    /// Return the absolute output path.
    pub(crate) fn path(&self) -> &Path {
        &self.absolute_path
    }
}

/// One emitted output reference mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputReferenceKind {
    /// One static module import or re-export reference.
    Import,
    /// One runtime-facing document, chunk, or asset reference.
    Runtime,
}

/// One resolved output layout for one target.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OutputLayout<'a> {
    /// The package directory that anchors relative output paths.
    package_dir: &'a Path,
    /// The target whose outputs are being laid out.
    target: &'a Target,
}

impl<'a> OutputLayout<'a> {
    /// Create one output layout for one package target.
    pub(crate) fn new(package_dir: &'a Path, target: &'a Target) -> Self {
        Self {
            package_dir,
            target,
        }
    }

    /// Resolve one manifest output path for this target.
    pub(crate) fn manifest_location(&self) -> OutputLocation {
        let manifest_directory = if let Some(out_file) = self.target.out_file.as_ref() {
            let out_file = if out_file.is_absolute() {
                out_file.clone()
            } else {
                self.package_dir.join(out_file)
            };

            out_file.parent().unwrap_or(self.package_dir).to_path_buf()
        } else {
            self.target.resolve_out_dir(self.package_dir)
        };

        OutputLocation::new(manifest_directory.join(format!("{}.manifest.json", self.target.name)))
    }

    /// Resolve one linked HTML document path for this target.
    pub(crate) fn document_location(&self) -> OutputLocation {
        OutputLocation::new(self.default_target_output_path("html"))
    }

    /// Resolve one linked script entry path for this target.
    pub(crate) fn script_entry_location(&self) -> OutputLocation {
        self.script_named_entry_location(&self.target.name)
    }

    /// Resolve one linked script entry chunk path for one chunk name.
    pub(crate) fn script_named_entry_location(&self, chunk_name: &str) -> OutputLocation {
        if self.target.emit == destack_artifact::EmitFormat::Html {
            return OutputLocation::new(
                self.document_location()
                    .path()
                    .with_file_name(self.entry_file_name(chunk_name, "js")),
            );
        }

        if self.target.out_file.is_some() {
            return OutputLocation::new(self.default_target_output_path("js"));
        }

        OutputLocation::new(
            self.output_directory()
                .join(self.entry_file_name(chunk_name, "js")),
        )
    }

    /// Resolve one linked script shared chunk path for one chunk name.
    pub(crate) fn script_chunk_location(&self, chunk_name: &str) -> OutputLocation {
        OutputLocation::new(
            self.output_directory()
                .join(self.chunk_file_name(chunk_name, "js")),
        )
    }

    /// Resolve one standalone source map path next to one output file.
    pub(crate) fn linked_source_map_location(
        &self,
        output_location: &OutputLocation,
    ) -> OutputLocation {
        let extension = output_location
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}.map"))
            .unwrap_or_else(|| "map".to_string());

        OutputLocation::new(output_location.path().with_extension(extension))
    }

    /// Build one output location from one resolved output path.
    pub(crate) fn output_location(&self, output_path: PathBuf) -> OutputLocation {
        OutputLocation::new(output_path)
    }

    /// Return one manifest-visible path for one output location.
    pub(crate) fn manifest_path(&self, output_location: &OutputLocation) -> String {
        if let Some(relative) = self.output_relative_path(output_location.path()) {
            return self.normalize_output_path(&relative);
        }

        if let Ok(relative) = output_location.path().strip_prefix(self.package_dir) {
            return self.normalize_output_path(relative);
        }

        self.normalize_output_path(output_location.path())
    }

    /// Return one emitted reference from one output file to another.
    pub(crate) fn output_reference(
        &self,
        from_output: &OutputLocation,
        to_output: &OutputLocation,
        kind: OutputReferenceKind,
    ) -> String {
        match kind {
            OutputReferenceKind::Import => self.import_output_specifier(from_output, to_output),
            OutputReferenceKind::Runtime => self
                .public_output_specifier(to_output)
                .unwrap_or_else(|| self.import_output_specifier(from_output, to_output)),
        }
    }

    /// Resolve the absolute output directory for this target.
    fn output_directory(&self) -> PathBuf {
        self.target.resolve_out_dir(self.package_dir)
    }

    /// Resolve one default target output path for this target.
    fn default_target_output_path(&self, extension: &str) -> PathBuf {
        if let Some(out_file) = self.target.out_file.as_ref() {
            if out_file.is_absolute() {
                return out_file.clone();
            }

            return self.package_dir.join(out_file);
        }

        self.output_directory()
            .join(format!("{}.{}", self.target.name, extension))
    }

    /// Build one entry file name for this target.
    fn entry_file_name(&self, name: &str, extension: &str) -> String {
        self.render_output_file_name(
            self.target.bundle.output.entry_file_names.as_deref(),
            name,
            extension,
        )
    }

    /// Build one shared chunk file name for this target.
    fn chunk_file_name(&self, name: &str, extension: &str) -> String {
        self.render_output_file_name(
            self.target.bundle.output.chunk_file_names.as_deref(),
            name,
            extension,
        )
    }

    /// Render one configured output file name template.
    fn render_output_file_name(
        &self,
        template: Option<&str>,
        name: &str,
        extension: &str,
    ) -> String {
        let extension_with_dot = format!(".{extension}");
        let template = template.unwrap_or("[name].[ext]");

        template
            .replace("[name]", name)
            .replace("[extname]", &extension_with_dot)
            .replace("[ext]", extension)
    }

    /// Return one public output specifier when the target config asks for it.
    fn public_output_specifier(&self, output_location: &OutputLocation) -> Option<String> {
        let public_path = self.target.bundle.output.public_path.as_deref()?;
        let relative = self.output_relative_path(output_location.path())?;
        let relative = self.normalize_output_path(&relative);

        Some(join_public_path(public_path, &relative))
    }

    /// Return one output-relative path when the target output lives under the output directory.
    fn output_relative_path(&self, output_path: &Path) -> Option<PathBuf> {
        output_path
            .strip_prefix(self.output_directory())
            .ok()
            .map(Path::to_path_buf)
    }

    /// Return one import specifier from one output location to another.
    fn import_output_specifier(
        &self,
        from_output: &OutputLocation,
        to_output: &OutputLocation,
    ) -> String {
        let relative = self.relative_output_path_between(from_output.path(), to_output.path());
        let relative = self.normalize_output_path(&relative);

        if relative.is_empty() {
            return ".".to_string();
        }

        if relative.starts_with('.') {
            return relative;
        }

        format!("./{relative}")
    }

    /// Return one relative output path from one emitted file to another.
    fn relative_output_path_between(
        &self,
        from_output_path: &Path,
        to_output_path: &Path,
    ) -> PathBuf {
        let from_directory = from_output_path.parent().unwrap_or_else(|| Path::new(""));
        let from_components = from_directory.components().collect::<Vec<_>>();
        let to_components = to_output_path.components().collect::<Vec<_>>();
        let mut shared = 0;

        while shared < from_components.len()
            && shared < to_components.len()
            && from_components[shared] == to_components[shared]
        {
            shared += 1;
        }

        let mut relative_path = PathBuf::new();

        for component in &from_components[shared..] {
            if matches!(component, Component::Normal(_)) {
                relative_path.push("..");
            }
        }

        for component in &to_components[shared..] {
            if let Component::Normal(segment) = component {
                relative_path.push(segment);
            }
        }

        relative_path
    }

    /// Normalize one output path for manifests and specifiers.
    fn normalize_output_path(&self, path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Join one public path prefix and one output-relative path.
fn join_public_path(public_path: &str, relative_path: &str) -> String {
    let public_path = public_path.trim_end_matches('/');
    let relative_path = relative_path.trim_start_matches('/');

    if public_path.is_empty() {
        return format!("/{relative_path}");
    }

    format!("{public_path}/{relative_path}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use destack_workspace::Target;

    use super::{OutputLayout, OutputReferenceKind};

    /// Prefer the configured public path for runtime-facing output references.
    #[test]
    fn test_uses_public_path_for_runtime_output_reference() {
        let mut target = Target::html("site");
        target.bundle.output.public_path = Some("/static".to_string());

        let layout = OutputLayout::new(Path::new("/workspace/pkg"), &target);
        let document_path = layout.document_location();
        let entry_path = layout.script_entry_location();

        assert_eq!(
            layout.output_reference(&document_path, &entry_path, OutputReferenceKind::Runtime),
            "/static/site.js"
        );
    }

    /// Render configured entry file name templates for assembled script output.
    #[test]
    fn test_renders_entry_file_name_template_for_script_entry() {
        let mut target = Target::js("app");
        target.bundle.output.entry_file_names = Some("entries/[name]-bundle.[ext]".to_string());

        let layout = OutputLayout::new(Path::new("/workspace/pkg"), &target);

        assert_eq!(
            layout.script_entry_location().path(),
            Path::new("/workspace/pkg/dist/entries/app-bundle.js")
        );
    }

    /// Render configured chunk file name templates for shared chunks.
    #[test]
    fn test_renders_chunk_file_name_template_for_script_chunk() {
        let mut target = Target::js("app");
        target.bundle.output.chunk_file_names = Some("chunks/[name]-shared.[ext]".to_string());

        let layout = OutputLayout::new(Path::new("/workspace/pkg"), &target);

        assert_eq!(
            layout.script_chunk_location("shared-value").path(),
            Path::new("/workspace/pkg/dist/chunks/shared-value-shared.js")
        );
    }
}
