use std::collections::{BTreeMap, HashSet};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileId, Uri};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::config::parse_json_file;

/// Parsed `package.json` declaration.
#[derive(Debug, Clone)]
pub struct PackageDeclaration {
    /// The id of the `package.json` file.
    pub file_id: FileId,
    /// The uri of the `package.json` file.
    pub uri: Uri,
    /// The physical path to the `package.json` file.
    pub path: PathBuf,
    /// The declaration directory.
    pub directory: PathBuf,
    /// The manifest fields needed from the declaration.
    pub manifest: PackageManifest,
}

/// The subset of `package.json` fields used by package declaration consumers.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    /// The package name.
    pub name: Option<String>,
    /// The package version.
    pub version: Option<String>,
    /// The module type field.
    #[serde(rename = "type")]
    pub module_type: Option<String>,
    /// The `main` entry point.
    pub main: Option<String>,
    /// The `module` entry point.
    pub module: Option<String>,
    /// The `types` entry point.
    pub types: Option<String>,
    /// The `bin` field.
    pub bin: Option<Value>,
    /// The browser mapping.
    pub browser: Option<Value>,
    /// The exports mapping.
    pub exports: Option<Value>,
    /// The imports mapping.
    pub imports: Option<Map<String, Value>>,
    /// The script entries.
    pub scripts: Option<BTreeMap<String, String>>,
}

impl PackageDeclaration {
    /// Parse one package declaration from one json file.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        let manifest: PackageManifest = serde_json::from_value(parse_json_file(file)?)?;
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
        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "package.json must have a parent directory",
            ))
        })?;

        Ok(Self {
            file_id: file.id,
            uri: file.uri.clone(),
            path,
            directory,
            manifest,
        })
    }

    /// Return the declared package name when present.
    pub fn name(&self) -> Option<&str> {
        self.manifest.name.as_deref()
    }

    /// Return the declared package version when present.
    pub fn version(&self) -> Option<&str> {
        self.manifest.version.as_deref()
    }

    /// Return the declared package module type when present.
    pub fn module_type(&self) -> Option<&str> {
        self.manifest.module_type.as_deref()
    }
}

impl PackageManifest {
    /// Collect package entry targets in stable field order.
    pub fn entry_targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        let mut seen = HashSet::new();

        Self::collect_string_target(self.main.as_deref(), &mut targets, &mut seen);
        Self::collect_string_target(self.module.as_deref(), &mut targets, &mut seen);
        Self::collect_string_target(self.types.as_deref(), &mut targets, &mut seen);

        if let Some(bin) = self.bin.as_ref() {
            Self::collect_target_values(bin, &mut targets, &mut seen);
        }
        if let Some(exports) = self.exports.as_ref() {
            Self::collect_target_values(exports, &mut targets, &mut seen);
        }

        targets
    }

    /// Collect one optional string target.
    fn collect_string_target(
        value: Option<&str>,
        targets: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        let Some(value) = value else {
            return;
        };

        let target = value.trim();
        if target.is_empty() {
            return;
        }

        let target = target.to_string();
        if seen.insert(target.clone()) {
            targets.push(target);
        }
    }

    /// Collect nested string targets from one json value.
    fn collect_target_values(value: &Value, targets: &mut Vec<String>, seen: &mut HashSet<String>) {
        match value {
            Value::String(target) => {
                Self::collect_string_target(Some(target.as_str()), targets, seen);
            }
            Value::Array(values) => {
                for item in values {
                    Self::collect_target_values(item, targets, seen);
                }
            }
            Value::Object(entries) => {
                for item in entries.values() {
                    Self::collect_target_values(item, targets, seen);
                }
            }
            _ => {}
        }
    }
}
