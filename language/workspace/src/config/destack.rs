use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileId};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::{
    CompilerOptions, ConditionCatalog, ConditionalDependencies, Dependency, DiagnosticPolicy,
    Export, FormatterOptions, LinterOptions, PackagePatch, Policy, Product, ProfileOptions,
    RuntimeOptions, Target, Task, Topology, Vendor, builtin_modes, builtin_roles, parse_jsonc_file,
};

/// Destack configuration document.
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(title = "Destack"))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Destack {
    /// Package name.
    pub name: Option<String>,
    /// Package version.
    pub version: Option<String>,
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
    /// Repository wide workspace package and group configuration.
    workspace: Option<WorkspaceLayout>,
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
    /// Runtime configuration.
    pub runtime: RuntimeOptions,
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

impl Destack {
    /// Return the declared `extends` specifiers in order.
    pub(crate) fn extends(&self) -> impl Iterator<Item = &str> {
        self.extends.iter().map(String::as_str)
    }

    /// Complete derived config fields after deserialization.
    pub(crate) fn finish(&mut self) {
        let mut modes = builtin_modes();
        modes.extend(std::mem::take(&mut self.conditions.modes));
        self.conditions.modes = modes;

        let mut roles = builtin_roles();
        roles.extend(std::mem::take(&mut self.conditions.roles));
        self.conditions.roles = roles;
    }
}

/// Return the JSON schema for `destack.json`.
#[cfg(feature = "schema")]
pub fn destack_schema() -> schemars::Schema {
    schemars::schema_for!(Destack)
}

/// Workspace package layout.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct WorkspaceLayout {
    /// Package root glob patterns.
    packages: Option<Vec<String>>,
    /// Named groups of package paths.
    groups: Option<IndexMap<String, Vec<String>>>,
}

/// Loaded `destack.json` file.
#[derive(Debug, Clone)]
pub struct DestackFile {
    /// The id of the `destack.json` file.
    pub file_id: FileId,
    /// The declaration files used to build this effective declaration.
    pub file_ids: Vec<FileId>,
    /// Path to the `destack.json` file.
    pub path: PathBuf,
    /// The directory containing the `destack.json` file.
    pub directory: PathBuf,
    /// The effective parsed declaration.
    pub destack: Destack,
    /// The effective declaration source.
    source: Value,
}

impl std::ops::Deref for DestackFile {
    type Target = Destack;

    fn deref(&self) -> &Self::Target {
        &self.destack
    }
}

impl DestackFile {
    /// Parse one `destack.json` configuration from one file.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .ok_or_else(|| {
                serde_json::Error::io(Error::new(
                    ErrorKind::InvalidData,
                    "destack.json must have a valid path",
                ))
            })?;
        let source = parse_jsonc_file(file)?;

        Self::from_file(file.id, vec![file.id], path, source)
    }

    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.destack.extends()
    }

    /// Return explicit workspace package root patterns.
    pub fn workspace_packages(&self) -> Option<&[String]> {
        self.destack
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.packages.as_deref())
    }

    /// Return workspace member groups.
    pub fn workspace_groups(&self) -> Option<&IndexMap<String, Vec<String>>> {
        self.destack
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.groups.as_ref())
    }

    /// Inherit settings from one parent configuration.
    pub fn extend_from(&mut self, parent: &Self) -> Result<(), serde_json::Error> {
        let mut source = Self::merge_source(&parent.source, &self.source);
        let destack: Destack = serde_json::from_value(source.clone())?;
        let compiler = destack.compiler.clone();
        let file_ids = merge_file_ids(&parent.file_ids, &self.file_ids);

        // preserve monotonic compiler restrictions
        Self::apply_parent_restrictions(&mut source, &compiler, &parent.compiler)?;

        *self = Self::from_file(self.file_id, file_ids, self.path.clone(), source)?;

        Ok(())
    }

    /// Build one configuration from an effective source value.
    fn from_file(
        file_id: FileId,
        file_ids: Vec<FileId>,
        path: PathBuf,
        source: Value,
    ) -> Result<Self, serde_json::Error> {
        let mut file: Destack = serde_json::from_value(source.clone())?;

        file.finish();
        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "destack.json must have a parent directory",
            ))
        })?;

        Ok(Self {
            file_id,
            file_ids,
            path,
            directory,
            source,
            destack: file,
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
            "exclusiveMutableBorrows",
            compiler.restrictions.exclusive_mutable_borrows,
            parent.restrictions.exclusive_mutable_borrows,
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
        if !parent_policy.is_stricter_than(compiler_policy) {
            return Ok(());
        }

        let Some(source) = source.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective destack declaration must be an object",
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
