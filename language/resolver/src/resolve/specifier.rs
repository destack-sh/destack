use std::path::{Component, Path};

use destack_source::ModuleSpecifier;

/// One parsed specifier path for a resolve query.
#[derive(Debug, Clone)]
pub(crate) struct ResolverSpecifier {
    /// The specifier path without any query or fragment.
    pub(crate) path: String,
    /// The query suffix, including the leading `?`.
    pub(crate) query: Option<String>,
    /// The fragment suffix, including the leading `#`.
    pub(crate) fragment: Option<String>,
}

/// The coarse specifier class used to select one resolution path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolverSpecifierKind {
    /// One absolute filesystem path request.
    Absolute,
    /// One relative filesystem path request.
    Relative,
    /// One `#` package import request.
    PackageImport,
    /// One bare package or module request.
    Bare,
}

impl ResolverSpecifier {
    /// Parse one raw specifier into path, query, and fragment.
    pub(crate) fn parse(specifier: &str) -> Self {
        let parsed = ModuleSpecifier::parse(specifier);

        Self {
            path: parsed.path().to_string(),
            query: parsed.query,
            fragment: parsed.fragment,
        }
    }

    /// Return one candidate where the fragment is treated as part of the path.
    pub(crate) fn fragment_path_candidate(&self) -> Option<String> {
        if self.query.is_some() {
            return None;
        }

        self.fragment
            .as_ref()
            .map(|fragment| format!("{}{fragment}", self.path))
    }

    /// Classify one specifier path without query or fragment.
    pub(crate) fn kind_for(specifier: &str) -> ResolverSpecifierKind {
        match Path::new(specifier).components().next() {
            Some(Component::RootDir | Component::Prefix(_)) => ResolverSpecifierKind::Absolute,
            Some(Component::CurDir | Component::ParentDir) => ResolverSpecifierKind::Relative,
            Some(Component::Normal(_)) if specifier.as_bytes()[0] == b'#' => {
                ResolverSpecifierKind::PackageImport
            }
            _ => ResolverSpecifierKind::Bare,
        }
    }
}
