/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFormat {
    /// `.ds`
    Dyst,
    /// `.d.ds`
    DystDeclaration,
    /// `.dst`
    DystText,
    /// `.dsb`
    DystBinary,
    /// `.dsx`
    DystExecutable,
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
}

impl SourceFormat {
    /// Get a source format from a file extension.
    pub fn from_extension(s: &str) -> Option<Self> {
        match s {
            "ds" => Some(SourceFormat::Dyst),
            "d.ds" => Some(SourceFormat::DystDeclaration),
            "dst" => Some(SourceFormat::DystText),
            "dsb" => Some(SourceFormat::DystBinary),
            "dsx" => Some(SourceFormat::DystExecutable),
            "js" => Some(SourceFormat::JavaScript),
            "jsx" => Some(SourceFormat::JavaScriptXml),
            "ts" => Some(SourceFormat::TypeScript),
            "tsx" => Some(SourceFormat::TypeScriptXml),
            "d.ts" => Some(SourceFormat::TypeScriptDeclaration),
            _ => None,
        }
    }
}

impl SourceFormat {
    /// Get the file extension for a source format.
    pub fn extension(&self) -> &str {
        match self {
            SourceFormat::Dyst => "ds",
            SourceFormat::DystDeclaration => "d.ds",
            SourceFormat::DystText => "dst",
            SourceFormat::DystBinary => "dsb",
            SourceFormat::DystExecutable => "dsx",
            SourceFormat::JavaScript => "js",
            SourceFormat::JavaScriptXml => "jsx",
            SourceFormat::TypeScript => "ts",
            SourceFormat::TypeScriptXml => "tsx",
            SourceFormat::TypeScriptDeclaration => "d.ts",
        }
    }

    /// Get the glob pattern for a source format.
    pub fn glob(&self) -> &str {
        match self {
            SourceFormat::Dyst => "**/*.ds",
            SourceFormat::DystDeclaration => "**/*.d.ds",
            SourceFormat::DystText => "**/*.dst",
            SourceFormat::DystBinary => "**/*.dsb",
            SourceFormat::DystExecutable => "**/*.dsx",
            SourceFormat::JavaScript => "**/*.js",
            SourceFormat::JavaScriptXml => "**/*.jsx",
            SourceFormat::TypeScript => "**/*.ts",
            SourceFormat::TypeScriptXml => "**/*.tsx",
            SourceFormat::TypeScriptDeclaration => "**/*.d.ts",
        }
    }
}
