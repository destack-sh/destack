use std::io::{Error, ErrorKind};
use std::mem;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tspp_artifact::Stability;
use tspp_source::{File, FileId, FileSystem, FileType, Uri, matches, matches_prefix};

use crate::RepositoryError;
use crate::config::{
    CompilerOptions, ConditionCatalog, ConditionalDependencies, DEFAULT_SOURCE_EXCLUDE,
    DEFAULT_SOURCE_INCLUDE, Dependency, DiagnosticPolicy, ExecutionOptions, Export,
    FormatterOptions, LinterOptions, PackagePatch, Policy, Product, ProfileOptions, Target, Task,
    Topology, Vendor, builtin_modes, builtin_roles, parse_jsonc_file,
};

/// File name of a TS++ package manifest.
pub const MANIFEST_FILE_NAME: &str = "package.json";
/// The `packageManager` prefix marking a `package.json` as a TS++ manifest.
const PACKAGE_MANAGER_PREFIX: &str = "tspp@";

/// One TS++ package manifest, read from `package.json`.
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(title = "TS++ package.json"))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// The toolchain this package requires, as `tspp@<version>`.
    pub package_manager: Option<String>,
    /// Package or workspace name.
    pub name: Option<String>,
    /// Package or workspace version.
    pub version: Option<String>,
    /// Stability promised by this package or workspace.
    pub stability: Option<Stability>,
    /// Whether the package is private.
    pub r#private: Option<bool>,
    /// Package description.
    pub description: Option<String>,
    /// Package license identifier.
    pub license: Option<String>,
    /// Package repository metadata.
    pub repository: Option<Value>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Package keywords.
    pub keywords: Vec<String>,
    /// Workspace package root glob patterns.
    pub workspaces: Option<Vec<String>>,
    /// Named groups of workspace package paths.
    pub groups: IndexMap<String, Vec<String>>,
    /// Config path inherited before this config.
    pub extends: Option<String>,
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Public package exports.
    pub exports: IndexMap<String, Export>,
    /// Package dependencies.
    pub dependencies: IndexMap<String, Dependency>,
    /// Dependencies enabled by condition predicates.
    pub conditional_dependencies: Vec<ConditionalDependencies>,
    /// Package dependency overrides.
    pub overrides: IndexMap<String, Dependency>,
    /// Package patch files.
    pub patches: IndexMap<String, PackagePatch>,
    /// Vendored dependency resolution declaration.
    pub vendor: Vendor,
    /// Package topology definition.
    pub topology: Topology,
    /// Compiler configuration.
    pub compiler: CompilerOptions,
    /// Package policy declarations and rules.
    pub policy: Policy,
    /// World and Runtime execution configuration.
    pub execution: ExecutionOptions,
    /// Formatter configuration.
    pub formatter: FormatterOptions,
    /// Linter configuration.
    pub linter: LinterOptions,
    /// Build targets.
    pub targets: IndexMap<String, Target>,
    /// Deliverable products.
    pub products: IndexMap<String, Product>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileOptions>,
    /// Named source graph conditions.
    pub conditions: ConditionCatalog,
    /// Named toolchain and shell tasks.
    pub tasks: IndexMap<String, Task>,
    /// Default target for the package.
    pub default_target: Option<String>,
    /// Default product for the package.
    pub default_product: Option<String>,
}

impl Manifest {
    /// Return the declared `extends` specifiers in order.
    pub(crate) fn extends(&self) -> impl Iterator<Item = &str> {
        self.extends.iter().map(String::as_str)
    }

    /// Return whether one JSON document is a TS++ manifest by its `packageManager` marker.
    pub fn is_marked(source: &Value) -> bool {
        source
            .get("packageManager")
            .and_then(Value::as_str)
            .is_some_and(|marker| marker.starts_with(PACKAGE_MANAGER_PREFIX))
    }

    /// Return whether this workspace selects one package root.
    pub fn selects_package(&self, path: &str) -> bool {
        let path = if path.is_empty() { "." } else { path };
        let Some(patterns) = self.workspaces.as_deref() else {
            return path == ".";
        };

        patterns
            .iter()
            .any(|pattern| matches(pattern.as_bytes(), path.as_bytes()))
    }

    /// Return whether this workspace selects a package root below one directory.
    pub fn selects_package_below(&self, path: &str) -> bool {
        let path = if path.is_empty() { "." } else { path };
        let Some(patterns) = self.workspaces.as_deref() else {
            return false;
        };

        // require descendant matches to cross a path separator
        let prefix = if path == "." {
            String::new()
        } else {
            format!("{path}/")
        };

        patterns
            .iter()
            .any(|pattern| matches_prefix(pattern.as_bytes(), prefix.as_bytes()))
    }

    /// Complete derived config fields after deserialization.
    pub(crate) fn finish(&mut self) {
        // canonicalize package source paths and patterns once
        let patterns = self
            .files
            .iter_mut()
            .chain(self.include.iter_mut())
            .chain(self.exclude.iter_mut());
        for pattern in patterns {
            *pattern = pattern
                .replace('\\', "/")
                .trim_start_matches("./")
                .to_string();
        }

        // canonicalize workspace package patterns once
        if let Some(packages) = self.workspaces.as_mut() {
            for pattern in packages {
                *pattern = pattern
                    .replace('\\', "/")
                    .trim_start_matches("./")
                    .to_string();
            }
        }

        // canonicalize unordered code selections
        for target in self.targets.values_mut() {
            target.code.sort_unstable();
            target.code.dedup();
        }

        // install builtin conditions before user declarations
        let mut modes = builtin_modes();
        modes.extend(mem::take(&mut self.conditions.modes));
        self.conditions.modes = modes;

        let mut roles = builtin_roles();
        roles.extend(mem::take(&mut self.conditions.roles));
        self.conditions.roles = roles;
    }
}

/// Return the JSON schema for a TS++ `package.json`.
#[cfg(feature = "schema")]
pub fn manifest_schema() -> schemars::Schema {
    schemars::schema_for!(Manifest)
}

/// One loaded manifest file.
#[derive(Debug, Clone)]
pub struct ManifestFile {
    /// The id of the `package.json` file.
    pub file_id: FileId,
    /// The declaration files used to build this effective declaration.
    pub file_ids: Vec<FileId>,
    /// Path to the `package.json` file.
    pub path: PathBuf,
    /// The directory containing the `package.json` file.
    pub directory: PathBuf,
    /// The effective parsed declaration.
    pub manifest: Manifest,
    /// The effective declaration source.
    source: Value,
}

impl Deref for ManifestFile {
    type Target = Manifest;

    fn deref(&self) -> &Self::Target {
        &self.manifest
    }
}

impl ManifestFile {
    /// Read one TS++ manifest from a physical directory, none when absent or unmarked.
    pub(crate) fn read(
        file_system: &dyn FileSystem,
        root: &Path,
    ) -> Result<Option<Self>, RepositoryError> {
        let path = root.join(MANIFEST_FILE_NAME);

        // read config when present
        let content = match file_system.read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(RepositoryError::FileSystem {
                    operation: "read_to_string",
                    path,
                    message: error.to_string(),
                });
            }
        };

        // parse through normal repository config logic
        let file_id = FileId::from_logical_str(MANIFEST_FILE_NAME);
        let file = File::from_text(
            file_id,
            MANIFEST_FILE_NAME.to_string(),
            Uri::from_path(&path),
            Some(path.clone()),
            FileType::Json,
            content,
        )
        .map_err(|error| RepositoryError::InvalidFile {
            file: file_id,
            message: error.to_string(),
        })?;
        let file = Arc::new(file);

        Self::parse(&file).map_err(|error| RepositoryError::InvalidConfigFile {
            path,
            message: error.to_string(),
        })
    }

    /// Parse one TS++ manifest from one file, none for a `package.json` of another package manager.
    pub fn parse(file: &Arc<File>) -> Result<Option<Self>, serde_json::Error> {
        let source = parse_jsonc_file(file)?;
        if !Manifest::is_marked(&source) {
            return Ok(None);
        }
        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .ok_or_else(|| {
                serde_json::Error::io(Error::new(
                    ErrorKind::InvalidData,
                    "package.json must have a valid path",
                ))
            })?;

        Self::from_file(file.id, vec![file.id], path, source).map(Some)
    }

    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.manifest.extends()
    }

    /// Return whether one package-relative path belongs to this package's source set.
    pub fn includes_source(&self, path: &str) -> bool {
        if self.excludes_source(path) {
            return false;
        }

        // accept exact declared files first
        let is_listed = self.files.iter().any(|file| file == path);
        if is_listed {
            return true;
        }

        // use conventional source roots only without an explicit selection
        if self.include.is_empty() && self.files.is_empty() {
            return DEFAULT_SOURCE_INCLUDE
                .iter()
                .any(|pattern| Self::matches_source_pattern(pattern, path));
        }

        self.include
            .iter()
            .any(|pattern| Self::matches_source_pattern(pattern, path))
    }

    /// Return whether one package-relative path is excluded from package source.
    pub fn excludes_source(&self, path: &str) -> bool {
        self.exclude
            .iter()
            .map(String::as_str)
            .chain(DEFAULT_SOURCE_EXCLUDE.iter().copied())
            .any(|pattern| Self::matches_source_pattern(pattern, path))
    }

    /// Return whether one source pattern contains one package-relative path.
    fn matches_source_pattern(pattern: &str, path: &str) -> bool {
        let directory_pattern = pattern.strip_suffix("/**");
        let is_match = matches(pattern.as_bytes(), path.as_bytes());
        let is_directory_match =
            directory_pattern.is_some_and(|pattern| matches(pattern.as_bytes(), path.as_bytes()));

        // include descendants selected through a literal directory path
        let is_descendant = path
            .strip_prefix(pattern)
            .is_some_and(|suffix| suffix.starts_with('/'));

        is_match || is_directory_match || is_descendant
    }

    /// Inherit settings from one parent configuration.
    pub fn extend_from(&mut self, parent: &Self) -> Result<(), serde_json::Error> {
        let mut source = Self::merge_source(&parent.source, &self.source);
        let manifest: Manifest = serde_json::from_value(source.clone())?;
        let compiler = manifest.compiler.clone();
        let file_ids = merge_file_ids(&parent.file_ids, &self.file_ids);

        // preserve monotonic compiler restrictions
        Self::apply_parent_restrictions(&mut source, &compiler, &parent.compiler)?;

        *self = Self::from_file(self.file_id, file_ids, self.path.clone(), source)?;

        Ok(())
    }

    /// Build one configuration from an effective source value.
    pub(crate) fn from_file(
        file_id: FileId,
        file_ids: Vec<FileId>,
        path: PathBuf,
        source: Value,
    ) -> Result<Self, serde_json::Error> {
        let mut file: Manifest = serde_json::from_value(source.clone())?;

        // complete derived configuration
        file.finish();
        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "package.json must have a parent directory",
            ))
        })?;

        Ok(Self {
            file_id,
            file_ids,
            path,
            directory,
            source,
            manifest: file,
        })
    }

    /// Merge one child source value over one parent source value.
    fn merge_source(parent: &Value, child: &Value) -> Value {
        let mut parent = parent.clone();

        // drop inheritance directives before merging
        if let Value::Object(parent) = &mut parent {
            parent.remove("extends");
        }

        Self::merge_json(&parent, child)
    }

    /// Merge one child file value over one parent file value.
    fn merge_json(parent: &Value, child: &Value) -> Value {
        match (parent, child) {
            (Value::Object(parent), Value::Object(child)) => {
                let mut merged = parent.clone();

                for (key, child_value) in child {
                    let merged_value = if let Some(parent_value) = merged.get(key) {
                        match key.as_str() {
                            "policy" => Self::merge_policy_json(parent_value, child_value),
                            "dependencies" | "overrides" | "exports" => {
                                Self::merge_map_json(parent_value, child_value)
                            }
                            "conditionalDependencies" => {
                                Self::merge_array_json(parent_value, child_value)
                            }
                            _ => Self::merge_json(parent_value, child_value),
                        }
                    } else {
                        child_value.clone()
                    };

                    merged.insert(key.clone(), merged_value);
                }

                Value::Object(merged)
            }
            _ => child.clone(),
        }
    }

    /// Append arrays when both declarations define them.
    fn merge_array_json(parent: &Value, child: &Value) -> Value {
        match (parent, child) {
            (Value::Array(parent), Value::Array(child)) => {
                let mut merged = Vec::with_capacity(parent.len() + child.len());
                merged.extend(parent.iter().cloned());
                merged.extend(child.iter().cloned());

                Value::Array(merged)
            }
            _ => child.clone(),
        }
    }

    /// Merge maps without merging individual entries.
    fn merge_map_json(parent: &Value, child: &Value) -> Value {
        match (parent, child) {
            (Value::Object(parent), Value::Object(child)) => {
                let mut merged = parent.clone();

                for (key, child_value) in child {
                    merged.insert(key.clone(), child_value.clone());
                }

                Value::Object(merged)
            }
            _ => child.clone(),
        }
    }

    /// Merge one child policy file value over one parent policy file value.
    fn merge_policy_json(parent: &Value, child: &Value) -> Value {
        let mut merged = Self::merge_json(parent, child);

        // append policy declarations instead of replacing them
        if let Value::Object(merged) = &mut merged {
            Self::merge_policy_array(merged, parent, child, "requires");
            Self::merge_policy_array(merged, parent, child, "rules");
        }

        merged
    }

    /// Merge one policy array when both declarations define it.
    fn merge_policy_array(
        merged: &mut serde_json::Map<String, Value>,
        parent: &Value,
        child: &Value,
        key: &str,
    ) {
        let Some(parent_items) = parent.get(key).and_then(Value::as_array) else {
            return;
        };
        let Some(child_items) = child.get(key).and_then(Value::as_array) else {
            return;
        };

        let mut items = Vec::with_capacity(parent_items.len() + child_items.len());
        items.extend(parent_items.iter().cloned());
        items.extend(child_items.iter().cloned());

        merged.insert(key.to_string(), Value::Array(items));
    }

    /// Apply parent restrictions that descendants cannot loosen.
    fn apply_parent_restrictions(
        source: &mut Value,
        compiler: &CompilerOptions,
        parent: &CompilerOptions,
    ) -> Result<(), serde_json::Error> {
        Self::apply_parent_restriction(
            source,
            "noManaged",
            compiler.restrictions.no_managed,
            parent.restrictions.no_managed,
        )?;
        Self::apply_parent_restriction(
            source,
            "noHeap",
            compiler.restrictions.no_heap,
            parent.restrictions.no_heap,
        )?;
        Self::apply_parent_restriction(
            source,
            "noRuntime",
            compiler.restrictions.no_runtime,
            parent.restrictions.no_runtime,
        )?;
        Self::apply_parent_restriction(
            source,
            "noUnsafe",
            compiler.restrictions.no_unsafe,
            parent.restrictions.no_unsafe,
        )?;
        Self::apply_parent_restriction(
            source,
            "noDynamicDispatch",
            compiler.restrictions.no_dynamic_dispatch,
            parent.restrictions.no_dynamic_dispatch,
        )?;
        Self::apply_parent_restriction(
            source,
            "noReflection",
            compiler.restrictions.no_reflection,
            parent.restrictions.no_reflection,
        )?;
        Self::apply_parent_restriction(
            source,
            "noUnwind",
            compiler.restrictions.no_unwind,
            parent.restrictions.no_unwind,
        )?;
        Self::apply_parent_restriction(
            source,
            "noAliasingMutableBorrows",
            compiler.restrictions.no_aliasing_mutable_borrows,
            parent.restrictions.no_aliasing_mutable_borrows,
        )?;
        Self::apply_parent_restriction(
            source,
            "noImplicitReceivers",
            compiler.restrictions.no_implicit_receivers,
            parent.restrictions.no_implicit_receivers,
        )?;

        Ok(())
    }

    /// Apply one parent restriction when it is stricter than the child.
    fn apply_parent_restriction(
        source: &mut Value,
        key: &str,
        compiler_policy: DiagnosticPolicy,
        parent_policy: DiagnosticPolicy,
    ) -> Result<(), serde_json::Error> {
        if parent_policy <= compiler_policy {
            return Ok(());
        }

        let Some(source) = source.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective manifest must be an object",
            )));
        };

        let compiler_json = source
            .entry("compiler")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        let Some(compiler_json) = compiler_json.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective compiler declaration must be an object",
            )));
        };

        let restrictions_json = compiler_json
            .entry("restrictions")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        let Some(restrictions_json) = restrictions_json.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective compiler restrictions declaration must be an object",
            )));
        };

        restrictions_json.insert(key.to_string(), policy_json_value(parent_policy));

        Ok(())
    }
}

/// Return one invalid config IO error.
fn invalid_config_error(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, message.into())
}

/// Merge declaration file ids in inherited order.
fn merge_file_ids(parent: &[FileId], child: &[FileId]) -> Vec<FileId> {
    let mut file_ids = Vec::with_capacity(parent.len() + child.len());

    for file_id in parent.iter().chain(child) {
        if !file_ids.contains(file_id) {
            file_ids.push(*file_id);
        }
    }

    file_ids
}

/// Return one diagnostic policy file value.
fn policy_json_value(policy: DiagnosticPolicy) -> Value {
    let value = match policy {
        DiagnosticPolicy::Allow => "allow",
        DiagnosticPolicy::Warn => "warn",
        DiagnosticPolicy::Deny => "deny",
    };

    Value::String(value.to_string())
}

/// Invocation override applied to one manifest value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestOverride {
    /// Manifest path, such as `compiler.target`.
    pub path: String,
    /// Override payload value.
    pub value: Value,
}

impl ManifestOverride {
    /// Return the validated path segments for this override.
    pub fn path_segments(&self) -> Result<Vec<&str>, String> {
        // reject empty paths
        if self.path.is_empty() {
            return Err("empty manifest override path".to_string());
        }

        // split path
        let segments = self.path.split('.').collect::<Vec<_>>();

        // reject empty path segments
        if segments.iter().any(|segment| segment.is_empty()) {
            return Err(format!("invalid manifest override path: {}", self.path));
        }

        Ok(segments)
    }
}

/// Apply one list of manifest overrides to one JSON value.
pub fn apply_manifest_overrides_to_json(
    json: &mut Value,
    overrides: &[ManifestOverride],
) -> Result<(), String> {
    // apply overrides in order
    for override_ in overrides {
        let path = override_.path_segments()?;
        apply_manifest_override_to_json(json, &path, &override_.value);
    }

    Ok(())
}

/// Apply one manifest override to one JSON path.
fn apply_manifest_override_to_json(target: &mut Value, path: &[&str], value: &Value) {
    ensure_json_object(target);

    let Value::Object(object) = target else {
        return;
    };

    if path.len() == 1 {
        let entry = object
            .entry(path[0].to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        merge_json_value(entry, value);
        return;
    }

    // descend into child object
    let child = object
        .entry(path[0].to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    apply_manifest_override_to_json(child, &path[1..], value);
}

/// Merge one override value into one JSON value.
fn merge_json_value(target: &mut Value, value: &Value) {
    match (target, value) {
        (Value::Object(target_object), Value::Object(value_object)) => {
            for (key, value) in value_object {
                let entry = target_object
                    .entry(key.clone())
                    .or_insert_with(|| Value::Object(Map::new()));
                merge_json_value(entry, value);
            }
        }
        (target, value) => *target = value.clone(),
    }
}

/// Ensure one JSON value is an object.
fn ensure_json_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}
