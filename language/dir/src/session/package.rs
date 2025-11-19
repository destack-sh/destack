use core::fmt;
use std::path::PathBuf;

use serde::de::Error;
use serde::Deserialize;
use serde_json::{Map, Value};

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
    /// Parse a package.json file from JSON bytes.
    pub fn parse(
        path: PathBuf,
        realpath: PathBuf,
        content: Vec<u8>,
    ) -> Result<Self, serde_json::Error> {
        // strip BOM - UTF-8 BOM is 3 bytes: 0xEF, 0xBB, 0xBF
        let json_bytes = if content.starts_with(b"\xEF\xBB\xBF") {
            &content[3..]
        } else {
            &content[..]
        };

        // check if content is empty(ish)
        if json_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
            return Err(serde_json::Error::custom("File is empty"));
        }

        // parse options
        let mut options: PackageOptions = serde_json::from_slice(json_bytes)?;
        options.path = path;
        options.realpath = realpath;
        options.directory = options.path.parent().unwrap().to_path_buf();

        Ok(options)
    }
}
