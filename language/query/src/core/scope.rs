use destack_source::Uri;

/// Query index scope needed before one query can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryScope {
    /// Module query index for one file-backed request.
    Module {
        /// The file URI requested by the query.
        uri: Uri,
    },
    /// Module and workspace query indexes for one file-backed request.
    ModuleAndWorkspace {
        /// The file URI requested by the query.
        uri: Uri,
    },
    /// Workspace query index for one profile.
    Workspace,
}

impl QueryScope {
    /// Return the file URI requested by this scope when it has one.
    pub fn uri(&self) -> Option<&Uri> {
        match self {
            Self::Module { uri } | Self::ModuleAndWorkspace { uri } => Some(uri),
            Self::Workspace => None,
        }
    }
}
