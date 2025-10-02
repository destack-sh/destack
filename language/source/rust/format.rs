/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFormat {
    Dyst,
    DystText,
    DystBinary,
    DystExecutable,
}

impl SourceFormat {
    /// Get a source format from a file extension.
    pub fn from_extension(s: &str) -> Option<Self> {
        match s {
            "ds" => Some(SourceFormat::Dyst),
            "dst" => Some(SourceFormat::DystText),
            "dsb" => Some(SourceFormat::DystBinary),
            "dsx" => Some(SourceFormat::DystExecutable),
            _ => None,
        }
    }
}

impl SourceFormat {
    /// Get the file extension for a source format.
    pub fn extension(&self) -> &str {
        match self {
            SourceFormat::Dyst => "ds",
            SourceFormat::DystText => "dst",
            SourceFormat::DystBinary => "dsb",
            SourceFormat::DystExecutable => "dsx",
        }
    }

    /// Get the glob pattern for a source format.
    pub fn glob(&self) -> &str {
        match self {
            SourceFormat::Dyst => "**/*.ds",
            SourceFormat::DystText => "**/*.dst",
            SourceFormat::DystBinary => "**/*.dsb",
            SourceFormat::DystExecutable => "**/*.dsx",
        }
    }
}
