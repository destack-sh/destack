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
    /// `.dsx`
    DestackExecutable,

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
    /// Wasm.
    Wasm,

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
            "dsx" => FileType::DestackExecutable,

            "js" => FileType::JavaScript,
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

            _ => return None,
        };

        Some(ty)
    }

    /// Get the file extension from an extension or unknown.
    pub fn from_extension_or_unknown(s: &str) -> Self {
        Self::from_extension(s).unwrap_or(FileType::Unknown)
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
            FileType::DestackExecutable => "dsx",

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
            FileType::Wasm => "wasm",

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
            FileType::DestackExecutable => "**/*.dsx",

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
            FileType::Wasm => "**/*.wasm",

            FileType::Unknown => return None,
        };
        Some(extension)
    }
}
