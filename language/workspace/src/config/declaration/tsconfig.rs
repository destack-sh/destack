use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{File, FileId, PathExt, Uri};

use crate::config::{TsConfigJson, TsConfigOptions, parse_jsonc_file};

/// Template variable for the config directory path (e.g. `${configDir}`).
/// <https://github.com/microsoft/TypeScript/pull/58042>
const TEMPLATE_VARIABLE: &str = "${configDir}";

/// Default include pattern when neither `files` nor `include` is specified.
const TSCONFIG_ALL_PATTERN: &str = "**/*";

/// TypeScript source file extensions recognized by tsconfig project matching.
const TYPESCRIPT_EXTENSIONS: [&str; 4] = ["ts", "tsx", "mts", "cts"];

/// JavaScript source file extensions recognized when `allowJs` is enabled.
const JAVASCRIPT_EXTENSIONS: [&str; 4] = ["js", "jsx", "mjs", "cjs"];

/// Parsed `tsconfig.json` declaration.
#[derive(Debug, Clone)]
pub struct TsConfigDeclaration {
    /// The id of the `tsconfig.json` file.
    pub file_id: FileId,
    /// Whether this is the root tsconfig in its context.
    pub is_root: bool,
    /// The URI of the `tsconfig.json` file.
    pub uri: Uri,
    /// Path to the `tsconfig.json` file (including the `tsconfig.json`).
    pub path: PathBuf,
    /// The directory containing the `tsconfig.json` file.
    pub directory: PathBuf,
    /// Base directory from which to resolve path aliases.
    pub paths_base: PathBuf,
    /// The raw JSON content of the `tsconfig.json` file.
    pub json: TsConfigJson,
}

impl TsConfigDeclaration {
    /// Parse a tsconfig from a File with JSON content.
    pub fn parse(is_root: bool, file: &Arc<File>) -> Result<Self, serde_json::Error> {
        // parse the tsconfig from the JSON value
        let tsconfig_json: TsConfigJson = serde_json::from_value(parse_jsonc_file(file)?)?;

        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .ok_or_else(|| {
                serde_json::Error::io(Error::new(
                    ErrorKind::InvalidData,
                    "tsconfig.json must have a valid path",
                ))
            })?;
        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "tsconfig.json must have a parent directory",
            ))
        })?;

        let tsconfig = Self {
            file_id: file.id,
            is_root,
            uri: file.uri.clone(),
            path,
            directory: directory.clone(),
            paths_base: directory,
            json: tsconfig_json,
        };
        Ok(tsconfig)
    }

    /// Returns the base path from which to resolve aliases.
    pub fn base_path(&self) -> &Path {
        self.json
            .compiler_options
            .base_url
            .as_deref()
            .unwrap_or_else(|| &self.directory)
    }

    /// Derive effective tsconfig options from this declaration.
    pub fn options(&self) -> TsConfigOptions {
        TsConfigOptions::from(&self.json)
    }

    /// Inherits settings from the given tsconfig into `self`.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub fn extend_from(&mut self, tsconfig: &Self) {
        // files
        if self.json.files.is_none()
            && let Some(files) = &tsconfig.json.files
        {
            self.json.files = Some(files.clone());
        }

        // include
        if self.json.include.is_none()
            && let Some(include) = &tsconfig.json.include
        {
            self.json.include = Some(include.clone());
        }

        // exclude
        if self.json.exclude.is_none()
            && let Some(exclude) = &tsconfig.json.exclude
        {
            self.json.exclude = Some(exclude.clone());
        }

        let compiler_options = &mut self.json.compiler_options;

        // compilerOptions.baseUrl
        if compiler_options.base_url.is_none()
            && let Some(base_url) = &tsconfig.json.compiler_options.base_url
        {
            compiler_options.base_url = Some(
                if base_url.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                    base_url.clone()
                } else {
                    tsconfig.directory.join(base_url).normalize()
                },
            );
        }

        // compilerOptions.paths
        if compiler_options.paths.is_none() {
            self.paths_base = compiler_options.base_url.as_ref().map_or_else(
                || tsconfig.directory.to_path_buf(),
                |path| {
                    if path.to_string_lossy().starts_with(TEMPLATE_VARIABLE) {
                        path.clone()
                    } else {
                        tsconfig.directory.join(path).normalize()
                    }
                },
            );
            compiler_options.paths = tsconfig.json.compiler_options.paths.clone();
        }

        // compilerOptions.experimentalDecorators
        if compiler_options.experimental_decorators.is_none()
            && let Some(experimental_decorators) =
                tsconfig.json.compiler_options.experimental_decorators
        {
            compiler_options.experimental_decorators = Some(experimental_decorators);
        }

        // compilerOptions.emitDecoratorMetadata
        if compiler_options.emit_decorator_metadata.is_none()
            && let Some(emit_decorator_metadata) =
                tsconfig.json.compiler_options.emit_decorator_metadata
        {
            compiler_options.emit_decorator_metadata = Some(emit_decorator_metadata);
        }

        // compilerOptions.useDefineForClassFields
        if compiler_options.use_define_for_class_fields.is_none()
            && let Some(use_define_for_class_fields) =
                tsconfig.json.compiler_options.use_define_for_class_fields
        {
            compiler_options.use_define_for_class_fields = Some(use_define_for_class_fields);
        }

        // compilerOptions.rewriteRelativeImportExtensions
        if compiler_options
            .rewrite_relative_import_extensions
            .is_none()
            && let Some(rewrite_relative_import_extensions) = tsconfig
                .json
                .compiler_options
                .rewrite_relative_import_extensions
        {
            compiler_options.rewrite_relative_import_extensions =
                Some(rewrite_relative_import_extensions);
        }

        // compilerOptions.jsx
        if compiler_options.jsx.is_none()
            && let Some(jsx) = &tsconfig.json.compiler_options.jsx
        {
            compiler_options.jsx = Some(jsx.clone());
        }

        // compilerOptions.jsxFactory
        if compiler_options.jsx_factory.is_none()
            && let Some(jsx_factory) = &tsconfig.json.compiler_options.jsx_factory
        {
            compiler_options.jsx_factory = Some(jsx_factory.clone());
        }

        // compilerOptions.jsxFragmentFactory
        if compiler_options.jsx_fragment_factory.is_none()
            && let Some(jsx_fragment_factory) = &tsconfig.json.compiler_options.jsx_fragment_factory
        {
            compiler_options.jsx_fragment_factory = Some(jsx_fragment_factory.clone());
        }

        // compilerOptions.jsxImportSource
        if compiler_options.jsx_import_source.is_none()
            && let Some(jsx_import_source) = &tsconfig.json.compiler_options.jsx_import_source
        {
            compiler_options.jsx_import_source = Some(jsx_import_source.clone());
        }

        // compilerOptions.verbatimModuleSyntax
        if compiler_options.verbatim_module_syntax.is_none()
            && let Some(verbatim_module_syntax) =
                tsconfig.json.compiler_options.verbatim_module_syntax
        {
            compiler_options.verbatim_module_syntax = Some(verbatim_module_syntax);
        }

        // compilerOptions.preserveValueImports
        if compiler_options.preserve_value_imports.is_none()
            && let Some(preserve_value_imports) =
                tsconfig.json.compiler_options.preserve_value_imports
        {
            compiler_options.preserve_value_imports = Some(preserve_value_imports);
        }

        // compilerOptions.importsNotUsedAsValues
        if compiler_options.imports_not_used_as_values.is_none()
            && let Some(imports_not_used_as_values) =
                &tsconfig.json.compiler_options.imports_not_used_as_values
        {
            compiler_options.imports_not_used_as_values = Some(imports_not_used_as_values.clone());
        }

        // compilerOptions.target
        if compiler_options.target.is_none()
            && let Some(target) = &tsconfig.json.compiler_options.target
        {
            compiler_options.target = Some(target.clone());
        }

        // compilerOptions.module
        if compiler_options.module.is_none()
            && let Some(module) = &tsconfig.json.compiler_options.module
        {
            compiler_options.module = Some(module.clone());
        }

        // compilerOptions.allowJs
        if compiler_options.allow_js.is_none()
            && let Some(allow_js) = tsconfig.json.compiler_options.allow_js
        {
            compiler_options.allow_js = Some(allow_js);
        }
    }

    /// "Build" the root tsconfig in place, resolving:
    /// * `{configDir}` template variable
    /// * `paths_base` for resolving paths alias
    /// * `baseUrl` to absolute path
    pub fn build(&mut self) {
        // only the root tsconfig requires path resolution
        if !self.is_root {
            return;
        }

        let config_dir = self.directory.to_path_buf();

        if let Some(base_url) = &self.json.compiler_options.base_url {
            // substitute template variable in `tsconfig.compilerOptions.baseUrl`
            let base_url = base_url
                .to_string_lossy()
                .strip_prefix(TEMPLATE_VARIABLE)
                .map_or_else(
                    || config_dir.normalize_with(base_url),
                    |stripped_path| config_dir.join(stripped_path.trim_start_matches('/')),
                );
            self.json.compiler_options.base_url = Some(base_url);
        }

        if let Some(paths_by_alias) = self.json.compiler_options.paths.as_mut() {
            // paths_base should use base_url if set, otherwise config dir
            if let Some(base_url) = &self.json.compiler_options.base_url {
                self.paths_base = base_url.clone();
            }

            // default to config dir if paths_base is still empty
            if self.paths_base.as_os_str().is_empty() {
                self.paths_base = config_dir.clone();
            }

            // substitute template variable in `tsconfig.compilerOptions.paths`
            for paths in paths_by_alias.values_mut() {
                for path in paths {
                    Self::substitute_template_variable(&config_dir, path);
                }
            }
        }
    }

    /// Return whether this tsconfig applies to one source path.
    pub fn applies_to_path(&self, path: &Path) -> bool {
        let normalized_path = path.normalize();

        // files take precedence over excludes
        if self.json.files.as_ref().is_some_and(|files| {
            files
                .iter()
                .any(|file| self.matches_tsconfig_file(file, &normalized_path))
        }) {
            return true;
        }

        // include defaults to all supported source files unless files is set
        let is_included = self.json.include.as_ref().map_or_else(
            || {
                if self.json.files.is_some() {
                    false
                } else {
                    self.matches_tsconfig_pattern(TSCONFIG_ALL_PATTERN, &normalized_path)
                }
            },
            |include| {
                include
                    .iter()
                    .any(|pattern| self.matches_tsconfig_pattern(pattern, &normalized_path))
            },
        );

        if !is_included {
            return false;
        }

        // excludes only apply after the path was included
        self.json.exclude.as_ref().is_none_or(|exclude| {
            !exclude
                .iter()
                .any(|pattern| self.matches_tsconfig_pattern(pattern, &normalized_path))
        })
    }

    /// Resolves the given `specifier` within the project configured by this
    /// tsconfig, relative to the given `path`.
    pub fn resolve(
        &self,
        path: &Path,
        specifier: &str,
        resolve_reference: impl Fn(&Path) -> Option<TsConfigDeclaration>,
    ) -> Vec<PathBuf> {
        let paths = self.json.resolve_path_alias(specifier, &self.paths_base);

        for reference in &self.json.references {
            let reference_path = self.directory.normalize_with(&reference.path);
            let Some(tsconfig) = resolve_reference(&reference_path) else {
                continue;
            };

            if path.starts_with(tsconfig.base_path()) {
                let reference_paths = tsconfig
                    .json
                    .resolve_path_alias(specifier, &tsconfig.paths_base);
                return [reference_paths, paths].concat();
            }
        }

        paths
    }

    /// Template variable `${configDir}` for substitution of config files directory path.
    fn substitute_template_variable(directory: &Path, path: &mut String) {
        if let Some(stripped_path) = path.strip_prefix(TEMPLATE_VARIABLE) {
            *path = directory
                .join(stripped_path.trim_start_matches('/'))
                .to_string_lossy()
                .to_string();
        }
    }

    /// Return whether one `files` entry matches the given path exactly.
    fn matches_tsconfig_file(&self, file: &str, path: &Path) -> bool {
        self.resolve_tsconfig_path(file) == path
    }

    /// Return whether one include or exclude pattern matches the given path.
    fn matches_tsconfig_pattern(&self, pattern: &str, path: &Path) -> bool {
        if !self.is_supported_tsconfig_input(path) {
            return false;
        }

        let Some(relative_path) = path
            .strip_prefix(&self.directory)
            .ok()
            .map(Self::normalize_tsconfig_path)
        else {
            return false;
        };

        let normalized_pattern = self.resolve_tsconfig_pattern(pattern);
        if normalized_pattern == relative_path {
            return true;
        }

        if normalized_pattern.contains('*') || normalized_pattern.contains('?') {
            return Self::tsconfig_glob_matches_pattern(&normalized_pattern, &relative_path);
        }

        relative_path == normalized_pattern
            || relative_path.starts_with(&format!("{normalized_pattern}/"))
    }

    /// Resolve one tsconfig relative file entry to an absolute normalized path.
    fn resolve_tsconfig_path(&self, path: &str) -> PathBuf {
        let path = self.resolve_template_variable(path);
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path.normalize()
        } else {
            self.directory.normalize_with(path)
        }
    }

    /// Resolve one tsconfig pattern into a normalized relative or absolute pattern.
    fn resolve_tsconfig_pattern(&self, pattern: &str) -> String {
        let pattern = self.resolve_template_variable(pattern);
        let pattern = pattern.replace('\\', "/");
        let path = PathBuf::from(&pattern);

        if path.is_absolute() {
            path.normalize().to_string_lossy().to_string()
        } else {
            Self::normalize_tsconfig_path(Path::new(&pattern))
        }
    }

    /// Resolve the `${configDir}` template variable in one tsconfig path value.
    fn resolve_template_variable(&self, path: &str) -> String {
        path.strip_prefix(TEMPLATE_VARIABLE).map_or_else(
            || path.to_string(),
            |stripped_path| {
                self.directory
                    .join(stripped_path.trim_start_matches('/'))
                    .to_string_lossy()
                    .to_string()
            },
        )
    }

    /// Normalize one path for tsconfig glob matching.
    fn normalize_tsconfig_path(path: &Path) -> String {
        path.normalize()
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string()
    }

    /// Return whether one tsconfig input path has a supported source extension.
    fn is_supported_tsconfig_input(&self, path: &Path) -> bool {
        let allow_js = self
            .json
            .compiler_options
            .allow_js
            .is_some_and(|allow_js| allow_js);
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                TYPESCRIPT_EXTENSIONS.contains(&extension)
                    || allow_js && JAVASCRIPT_EXTENSIONS.contains(&extension)
            })
    }

    /// Return whether one normalized path matches one normalized tsconfig glob.
    fn tsconfig_glob_matches_pattern(pattern: &str, path: &str) -> bool {
        let pattern_segments = Self::tsconfig_path_segments(pattern);
        let path_segments = Self::tsconfig_path_segments(path);
        Self::tsconfig_glob_matches_segments(&pattern_segments, &path_segments)
    }

    /// Split one normalized path into path segments.
    fn tsconfig_path_segments(path: &str) -> Vec<&str> {
        path.split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    }

    /// Return whether one normalized path segment list matches one tsconfig glob.
    fn tsconfig_glob_matches_segments(pattern_segments: &[&str], path_segments: &[&str]) -> bool {
        if pattern_segments.is_empty() {
            return path_segments.is_empty();
        }

        // let `**` consume zero or more path segments
        if pattern_segments[0] == "**" {
            let mut rest_pattern_segments = &pattern_segments[1..];
            while rest_pattern_segments
                .first()
                .is_some_and(|segment| *segment == "**")
            {
                rest_pattern_segments = &rest_pattern_segments[1..];
            }

            if rest_pattern_segments.is_empty() {
                return true;
            }

            if Self::tsconfig_glob_matches_segments(rest_pattern_segments, path_segments) {
                return true;
            }

            if path_segments.is_empty() {
                return false;
            }

            return Self::tsconfig_glob_matches_segments(pattern_segments, &path_segments[1..]);
        }

        if path_segments.is_empty() {
            return false;
        }

        if !Self::tsconfig_segment_matches_pattern(pattern_segments[0], path_segments[0]) {
            return false;
        }

        Self::tsconfig_glob_matches_segments(&pattern_segments[1..], &path_segments[1..])
    }

    /// Return whether one path segment matches one tsconfig wildcard pattern.
    fn tsconfig_segment_matches_pattern(pattern_segment: &str, text_segment: &str) -> bool {
        let pattern_bytes = pattern_segment.as_bytes();
        let text_bytes = text_segment.as_bytes();
        let pattern_length = pattern_bytes.len();
        let text_length = text_bytes.len();

        let mut pattern_index = 0;
        let mut text_index = 0;
        let mut star_index = None;
        let mut match_index = 0;

        while text_index < text_length {
            if pattern_index < pattern_length
                && (pattern_bytes[pattern_index] == b'?'
                    || pattern_bytes[pattern_index] == text_bytes[text_index])
            {
                pattern_index += 1;
                text_index += 1;
                continue;
            }

            if pattern_index < pattern_length && pattern_bytes[pattern_index] == b'*' {
                star_index = Some(pattern_index);
                pattern_index += 1;
                match_index = text_index;
                continue;
            }

            let Some(star_index) = star_index else {
                return false;
            };

            pattern_index = star_index + 1;
            match_index += 1;
            text_index = match_index;
        }

        while pattern_index < pattern_length && pattern_bytes[pattern_index] == b'*' {
            pattern_index += 1;
        }

        pattern_index == pattern_length
    }
}
