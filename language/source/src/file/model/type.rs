use std::path::Path;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// The physical format of one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FileType {
    /// TS++ source code.
    Tspp,
    /// TS++ declaration source code.
    TsppDeclaration,
    /// JavaScript source code.
    #[serde(rename = "javascript")]
    JavaScript,
    /// Plain text.
    Text,
    /// TOML data.
    Toml,
    /// YAML data.
    Yaml,
    /// JSON data.
    Json,
    /// Dotenv data.
    Dotenv,
    /// HTML markup.
    Html,
    /// Markdown markup.
    Markdown,
    /// CSS styles.
    Css,
    /// SVG markup.
    Svg,
    /// WebAssembly bytecode.
    Wasm,
    /// A source map.
    SourceMap,
    /// A native object file.
    Object,
    /// Opaque bytes.
    Binary,
}

/// TS++ file types tried during extensionless module resolution.
pub const TSPP_FILE_TYPES: &[FileType] = &[FileType::Tspp, FileType::TsppDeclaration];

impl FileType {
    /// Return the file type for one extension when recognized.
    pub fn from_extension(extension: &str) -> Option<Self> {
        let file_type = match extension {
            // code
            "tspp" => Self::Tspp,
            "d.tspp" => Self::TsppDeclaration,
            "js" | "mjs" | "cjs" => Self::JavaScript,

            // data
            "txt" => Self::Text,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "json" => Self::Json,
            "env" => Self::Dotenv,

            // markup and styles
            "html" | "htm" => Self::Html,
            "md" => Self::Markdown,
            "css" => Self::Css,
            "svg" => Self::Svg,

            // executable formats
            "wasm" => Self::Wasm,
            "map" => Self::SourceMap,
            "o" => Self::Object,

            _ => return None,
        };

        Some(file_type)
    }

    /// Return the physical file type for one named path.
    pub fn from_path(path: &Path) -> Option<Self> {
        // recognize complete names before ordinary extensions
        let file_name = path.file_name()?;
        let file_name = file_name.to_str();
        match file_name {
            Some(file_name) if file_name == ".env" || file_name.starts_with(".env.") => {
                return Some(Self::Dotenv);
            }
            Some(file_name) if file_name.ends_with(".d.tspp") => {
                return Some(Self::TsppDeclaration);
            }
            _ => {}
        }

        // preserve every other named format as opaque bytes
        let file_type = path
            .extension()
            .and_then(|extension| extension.to_str())
            .and_then(Self::from_extension)
            .unwrap_or(Self::Binary);

        Some(file_type)
    }

    /// Return the canonical extension when the format has one.
    pub fn extension(self) -> Option<&'static str> {
        let extension = match self {
            Self::Tspp => "tspp",
            Self::TsppDeclaration => "d.tspp",
            Self::JavaScript => "js",
            Self::Text => "txt",
            Self::Toml => "toml",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Dotenv => "env",
            Self::Html => "html",
            Self::Markdown => "md",
            Self::Css => "css",
            Self::Svg => "svg",
            Self::Wasm => "wasm",
            Self::SourceMap => "map",
            Self::Object => "o",
            Self::Binary => return None,
        };

        Some(extension)
    }

    /// Return whether this format defaults to opaque bytes.
    pub fn is_opaque(self) -> bool {
        matches!(self, Self::Wasm | Self::Object | Self::Binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_file_paths() {
        let cases = [
            ("source.tspp", FileType::Tspp),
            ("types.d.tspp", FileType::TsppDeclaration),
            ("output.js", FileType::JavaScript),
            ("output.mjs", FileType::JavaScript),
            ("output.cjs", FileType::JavaScript),
            ("notes.txt", FileType::Text),
            ("package.toml", FileType::Toml),
            ("document.yaml", FileType::Yaml),
            ("document.yml", FileType::Yaml),
            ("document.json", FileType::Json),
            (".env", FileType::Dotenv),
            (".env.development.local", FileType::Dotenv),
            ("page.html", FileType::Html),
            ("page.htm", FileType::Html),
            ("readme.md", FileType::Markdown),
            ("style.css", FileType::Css),
            ("image.svg", FileType::Svg),
            ("module.wasm", FileType::Wasm),
            ("output.map", FileType::SourceMap),
            ("output.o", FileType::Object),
            ("image.png", FileType::Binary),
            ("shader.glsl", FileType::Binary),
            ("LICENSE", FileType::Binary),
        ];

        for (path, expected) in cases {
            assert_eq!(
                FileType::from_path(Path::new(path)),
                Some(expected),
                "{path}"
            );
        }
    }

    #[test]
    fn test_roundtrip_canonical_extensions() {
        let file_types = [
            FileType::Tspp,
            FileType::TsppDeclaration,
            FileType::JavaScript,
            FileType::Text,
            FileType::Toml,
            FileType::Yaml,
            FileType::Json,
            FileType::Dotenv,
            FileType::Html,
            FileType::Markdown,
            FileType::Css,
            FileType::Svg,
            FileType::Wasm,
            FileType::SourceMap,
            FileType::Object,
        ];

        for file_type in file_types {
            let extension = file_type
                .extension()
                .expect("canonical file type should have an extension");

            assert_eq!(FileType::from_extension(extension), Some(file_type));
        }
        assert_eq!(FileType::Binary.extension(), None);
    }
}
