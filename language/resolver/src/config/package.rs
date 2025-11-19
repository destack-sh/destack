use core::fmt;
use std::path::PathBuf;

use dyst_source::FileSystem;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::resolve::JSONError;

/// The package type.
/// <https://nodejs.org/api/packages.html#type>
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// CommonJS package.
    CommonJs,
    /// Module package.
    Module,
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommonJs => f.write_str("commonjs"),
            Self::Module => f.write_str("module"),
        }
    }
}

/// Package options (from `package.json`).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageOptions {
    /// Path to `package.json` (including the `package.json` filename).
    #[serde(skip)]
    pub path: PathBuf,

    /// Realpath of `package.json` (including the `package.json` filename).
    #[serde(skip)]
    pub realpath: PathBuf,

    /// Directory of `package.json` (excluding the `package.json` filename).
    #[serde(skip)]
    pub directory: PathBuf,

    /// Name of the package.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#name>
    pub name: Option<String>,

    /// Version of the package.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#version>
    pub version: Option<String>,

    /// Package type.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#type>
    pub r#type: Option<PackageType>,

    /// The "main" field.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#main>
    pub main: Option<String>,

    /// The "browser" field.
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub browser: Option<Value>,

    /// The "exports" field.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#exports>
    pub exports: Option<Value>,

    /// The "imports" field.
    /// <https://nodejs.org/api/packages.html?utm_source=chatgpt.com#imports>
    pub imports: Option<Map<String, Value>>,
}

impl fmt::Debug for PackageOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageOptions")
            .field("path", &self.path)
            .field("realpath", &self.realpath)
            .field("name", &self.name)
            .field("type", &self.r#type)
            .finish_non_exhaustive()
    }
}

impl PackageOptions {
    /// Parse a package.json file from JSON bytes
    pub fn parse<Fs: FileSystem>(
        _fs: &Fs,
        path: PathBuf,
        realpath: PathBuf,
        json: Vec<u8>,
    ) -> Result<Self, JSONError> {
        // strip BOM - UTF-8 BOM is 3 bytes: 0xEF, 0xBB, 0xBF
        let json_bytes = if json.starts_with(b"\xEF\xBB\xBF") {
            &json[3..]
        } else {
            &json[..]
        };

        // check if content is empty(ish)
        if json_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
            return Err(JSONError {
                path: path.clone(),
                message: "File is empty".to_string(),
                line: 0,
                column: 0,
            });
        }

        // parse options
        let mut options: PackageOptions =
            serde_json::from_slice(json_bytes).map_err(|error| JSONError {
                path: path.clone(),
                message: error.to_string(),
                line: error.line(),
                column: error.column(),
            })?;
        options.path = path;
        options.realpath = realpath;
        options.directory = options.path.parent().unwrap().to_path_buf();

        Ok(options)
    }
}

#[derive(Debug, Clone)]
pub struct ImportsExportsEntry<'a>(pub &'a Value);

impl<'a> ImportsExportsEntry<'a> {
    pub fn kind(&self) -> ImportsExportsKind {
        match self.0 {
            Value::String(_) => ImportsExportsKind::String,
            Value::Array(_) => ImportsExportsKind::Array,
            Value::Object(_) => ImportsExportsKind::Map,
            _ => ImportsExportsKind::Invalid,
        }
    }

    pub fn as_string(&self) -> Option<&'a str> {
        match self.0 {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<ImportsExportsArray<'a>> {
        match self.0 {
            Value::Array(arr) => Some(ImportsExportsArray(arr)),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<ImportsExportsMap<'a>> {
        match self.0 {
            Value::Object(obj) => Some(ImportsExportsMap(obj)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportsExportsArray<'a>(&'a [Value]);

impl<'a> ImportsExportsArray<'a> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ImportsExportsEntry<'a>> {
        ImportsExportsArrayIter {
            slice: self.0,
            index: 0,
        }
    }
}

struct ImportsExportsArrayIter<'a> {
    slice: &'a [Value],
    index: usize,
}

impl<'a> Iterator for ImportsExportsArrayIter<'a> {
    type Item = ImportsExportsEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.slice.get(self.index).map(|value| {
            self.index += 1;
            ImportsExportsEntry(value)
        })
    }
}

#[derive(Debug, Clone)]
pub struct ImportsExportsMap<'a>(pub(crate) &'a serde_json::Map<String, Value>);

impl<'a> ImportsExportsMap<'a> {
    pub(crate) fn get(&self, key: &str) -> Option<ImportsExportsEntry<'a>> {
        self.0.get(key).map(ImportsExportsEntry)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &'a str> {
        self.0.keys().map(String::as_str)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'a str, ImportsExportsEntry<'a>)> {
        self.0
            .iter()
            .map(|(k, v)| (k.as_str(), ImportsExportsEntry(v)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportsExportsKind {
    String,
    Array,
    Map,
    Invalid,
}
