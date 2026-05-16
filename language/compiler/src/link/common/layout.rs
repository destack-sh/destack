use std::path::{Component, Path, PathBuf};

use destack_workspace::{Module, Target};

use super::{OutputFileNameTemplate, OutputFileNameValues};

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

/// One target-scoped output location resolver.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TargetLocation<'a> {
    /// The package directory that anchors relative output paths.
    package_dir: &'a Path,
    /// The target whose outputs are being laid out.
    target: &'a Target,
    /// The target name used for default output names.
    target_name: &'a str,
}

impl<'a> TargetLocation<'a> {
    /// Create one output location resolver for one package target.
    pub(crate) fn new(package_dir: &'a Path, target: &'a Target, target_name: &'a str) -> Self {
        Self {
            package_dir,
            target,
            target_name,
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

        OutputLocation::new(manifest_directory.join(format!("{}.manifest.json", self.target_name)))
    }

    /// Resolve one linked HTML document path for this target.
    pub(crate) fn document_location(&self) -> OutputLocation {
        OutputLocation::new(self.default_target_output_path("html"))
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

    /// Return one import-style reference from one output file to another.
    pub(crate) fn output_reference(
        &self,
        from_output: &OutputLocation,
        to_output: &OutputLocation,
    ) -> String {
        self.import_output_specifier(from_output, to_output)
    }

    /// Return one runtime-visible reference from one output file to another.
    pub(crate) fn runtime_reference(
        &self,
        from_output: &OutputLocation,
        to_output: &OutputLocation,
    ) -> String {
        if let Some(public_path) = self.target.bundle_output.public_path.as_deref() {
            return join_public_output_path(public_path, &self.manifest_path(to_output));
        }

        self.output_reference(from_output, to_output)
    }

    /// Return one document-visible reference from one output file to another.
    pub(crate) fn document_reference(
        &self,
        from_output: &OutputLocation,
        to_output: &OutputLocation,
    ) -> String {
        self.runtime_reference(from_output, to_output)
    }

    /// Resolve the absolute output directory for this target.
    pub(crate) fn output_directory(&self) -> PathBuf {
        self.target.resolve_out_dir(self.package_dir)
    }

    /// Resolve one default target output path for this target.
    pub(crate) fn default_target_output_path(&self, extension: &str) -> PathBuf {
        if let Some(out_file) = self.target.out_file.as_ref() {
            if out_file.is_absolute() {
                return out_file.clone();
            }

            return self.package_dir.join(out_file);
        }

        self.output_directory()
            .join(format!("{}.{}", self.target_name, extension))
    }

    /// Render one configured output file name template with explicit token values.
    pub(crate) fn render_output_file_name_with_values(
        &self,
        template: Option<&str>,
        values: OutputFileNameValues<'_>,
    ) -> String {
        let template = OutputFileNameTemplate::new(template.unwrap_or("[name].[ext]"));

        template.render(values)
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
        let relative = relative_output_path_between(from_output.path(), to_output.path());
        let relative = self.normalize_output_path(&relative);

        if relative.is_empty() {
            return ".".to_string();
        }

        if relative.starts_with('.') {
            return relative;
        }

        format!("./{relative}")
    }

    /// Return one normalized output path string.
    fn normalize_output_path(&self, path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Return one relative output path from one emitted file to another.
fn relative_output_path_between(from_output_path: &Path, to_output_path: &Path) -> PathBuf {
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

/// Join one emitted output path onto one configured public path prefix.
fn join_public_output_path(public_path: &str, path: &str) -> String {
    let public_path = public_path.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    if public_path.is_empty() {
        return path.to_string();
    }

    if path.is_empty() {
        return public_path.to_string();
    }

    format!("{public_path}/{path}")
}

/// Return one stable source path for one module.
pub(crate) fn module_source_path(module: &Module) -> Result<PathBuf, String> {
    if let Some(path) = &module.path {
        return Ok(path.clone());
    }

    if let Some(path) = module.uri.to_path_buf() {
        return Ok(path);
    }

    Err(format!("module '{}' has no stable source path", module.uri))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use destack_workspace::Target;

    use super::TargetLocation;

    /// Render import references between emitted output files.
    #[test]
    fn test_render_output_reference_between_output_files() {
        let mut target = Target::html();
        target.out_dir = PathBuf::from("dist");
        let layout = TargetLocation::new(Path::new("/workspace/pkg"), &target, "site");
        let document_path =
            layout.output_location(Path::new("/workspace/pkg/dist/index.html").to_path_buf());
        let entry_path =
            layout.output_location(Path::new("/workspace/pkg/dist/site.js").to_path_buf());

        assert_eq!(
            layout.output_reference(&document_path, &entry_path),
            "./site.js"
        );
    }
}
