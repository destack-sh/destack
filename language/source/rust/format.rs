/// The format of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFormat {
    Dyst,
    DystText,
    DystBinary,
    DystExecutable,
}

impl SourceFormat {
    pub fn extension(&self) -> &str {
        match self {
            SourceFormat::Dyst => "ds",
            SourceFormat::DystText => "dst",
            SourceFormat::DystBinary => "dsb",
            SourceFormat::DystExecutable => "dsx",
        }
    }

    pub fn glob(&self) -> &str {
        match self {
            SourceFormat::Dyst => "**/*.ds",
            SourceFormat::DystText => "**/*.dst",
            SourceFormat::DystBinary => "**/*.dsb",
            SourceFormat::DystExecutable => "**/*.dsx",
        }
    }
}
