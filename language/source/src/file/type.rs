use std::path::Path;

/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// `.ds`
    Destack,
    /// `.d.ds`
    DestackDeclaration,
    /// `.dst`
    DestackText,
    /// `.dsb`
    DestackBinary,

    /// `.js`
    JavaScript,
    /// `.jsx`
    JavaScriptXml,
    /// `.ts`
    TypeScript,
    /// `.tsx`
    TypeScriptXml,
    /// `.d.ts`
    TypeScriptDeclaration,

    /// Text.
    Text,
    /// Toml,
    Toml,
    /// Yaml.
    Yaml,
    /// Json.
    Json,
    /// Html.
    Html,
    /// Markdown.
    Markdown,
    /// Wasm.
    Wasm,
    /// Node.
    Node,

    /// Source map (.map).
    SourceMap,
    /// Object file (.o).
    Object,

    /// Unknown.
    Unknown,
}

impl FileType {
    /// Get a source format from a file extension.
    pub fn from_extension(s: &str) -> Option<Self> {
        let ty = match s {
            "ds" => FileType::Destack,
            "d.ds" => FileType::DestackDeclaration,
            "dst" => FileType::DestackText,
            "dsb" => FileType::DestackBinary,

            "js" => FileType::JavaScript,
            "mjs" => FileType::JavaScript,
            "cjs" => FileType::JavaScript,
            "jsx" => FileType::JavaScriptXml,
            "ts" => FileType::TypeScript,
            "tsx" => FileType::TypeScriptXml,
            "d.ts" => FileType::TypeScriptDeclaration,

            "txt" => FileType::Text,
            "toml" => FileType::Toml,
            "yaml" => FileType::Yaml,
            "json" => FileType::Json,
            "html" => FileType::Html,
            "wasm" => FileType::Wasm,
            "node" => FileType::Node,
            "map" => FileType::SourceMap,
            "o" => FileType::Object,

            _ => return None,
        };

        Some(ty)
    }

    /// Get the file extension from an extension or unknown.
    pub fn from_extension_or_unknown(s: &str) -> Self {
        Self::from_extension(s).unwrap_or(FileType::Unknown)
    }

    /// Get a source format from a file path.
    pub fn from_path(path: &Path) -> Option<Self> {
        // detect compound extensions first
        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
            if file_name.ends_with(".d.ds") {
                return Some(FileType::DestackDeclaration);
            }
            if file_name.ends_with(".d.ts") {
                return Some(FileType::TypeScriptDeclaration);
            }
        }

        // fall back to the simple extension
        let extension = path.extension().and_then(|ext| ext.to_str());
        extension.and_then(FileType::from_extension)
    }

    /// Get a source format from a file path, defaulting to unknown.
    pub fn from_path_or_unknown(path: &Path) -> Self {
        Self::from_path(path).unwrap_or(FileType::Unknown)
    }
}

impl FileType {
    /// Get the file extension for a source format.
    pub fn extension(&self) -> Option<&str> {
        let extension = match self {
            FileType::Destack => "ds",
            FileType::DestackDeclaration => "d.ds",
            FileType::DestackText => "dst",
            FileType::DestackBinary => "dsb",

            FileType::JavaScript => "js",
            FileType::JavaScriptXml => "jsx",
            FileType::TypeScript => "ts",
            FileType::TypeScriptXml => "tsx",
            FileType::TypeScriptDeclaration => "d.ts",

            FileType::Text => "txt",
            FileType::Toml => "toml",
            FileType::Yaml => "yaml",
            FileType::Json => "json",
            FileType::Html => "html",
            FileType::Markdown => "md",
            FileType::Wasm => "wasm",
            FileType::Node => "node",
            FileType::SourceMap => "map",
            FileType::Object => "o",

            FileType::Unknown => return None,
        };
        Some(extension)
    }

    /// Get the glob pattern for a source format.
    pub fn glob(&self) -> Option<&str> {
        let extension = match self {
            FileType::Destack => "**/*.ds",
            FileType::DestackDeclaration => "**/*.d.ds",
            FileType::DestackText => "**/*.dst",
            FileType::DestackBinary => "**/*.dsb",

            FileType::JavaScript => "**/*.js",
            FileType::JavaScriptXml => "**/*.jsx",
            FileType::TypeScript => "**/*.ts",
            FileType::TypeScriptXml => "**/*.tsx",
            FileType::TypeScriptDeclaration => "**/*.d.ts",

            FileType::Text => "**/*.txt",
            FileType::Toml => "**/*.toml",
            FileType::Yaml => "**/*.yaml",
            FileType::Json => "**/*.json",
            FileType::Html => "**/*.html",
            FileType::Markdown => "**/*.md",
            FileType::Wasm => "**/*.wasm",
            FileType::Node => "**/*.node",
            FileType::SourceMap => "**/*.map",
            FileType::Object => "**/*.o",

            FileType::Unknown => return None,
        };
        Some(extension)
    }
}
