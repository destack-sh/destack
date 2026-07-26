use destack_source::ModuleSpecifier;

/// Parsed import module specifier.
pub(super) enum ImportSpecifier {
    /// Relative module path.
    Relative(ModuleSpecifier),
    /// Absolute module path.
    Absolute,
    /// Private import map specifier.
    Private,
    /// Internal module specifier.
    Internal,
    /// Scheme specifier.
    Scheme,
    /// Invalid package-like specifier.
    Invalid,
    /// External package export.
    Package(PackageSpecifier),
}

impl ImportSpecifier {
    /// Parse one import module specifier.
    pub(super) fn parse(specifier: &str) -> Self {
        let path = ModuleSpecifier::parse(specifier);
        let specifier_path = path.path().to_string();

        // relative module
        if specifier_path.starts_with("./") || specifier_path.starts_with("../") {
            Self::Relative(path)
        }
        // query or fragment
        else if path.query.is_some() || path.fragment.is_some() {
            Self::Invalid
        }
        // absolute module
        else if specifier_path.starts_with('/') {
            Self::Absolute
        }
        // private import map
        else if specifier_path.starts_with('#') {
            Self::Private
        }
        // internal module
        else if specifier_path.starts_with("destack:") || specifier_path.starts_with("destack://")
        {
            Self::Internal
        }
        // scheme specifier
        else if specifier_path.contains(':') {
            Self::Scheme
        }
        // package export or invalid package-like specifier
        else if let Some(specifier) = PackageSpecifier::parse(&specifier_path) {
            Self::Package(specifier)
        }
        // invalid
        else {
            Self::Invalid
        }
    }
}

/// Package export specifier.
pub(super) struct PackageSpecifier {
    /// Package name.
    pub(super) package: String,
    /// Export key inside the package.
    pub(super) export: String,
}

impl PackageSpecifier {
    /// Parse one package export specifier.
    fn parse(specifier: &str) -> Option<Self> {
        let mut parts = specifier.split('/').collect::<Vec<_>>();

        // reject empty path components
        if parts.iter().any(|part| part.is_empty()) {
            return None;
        }

        // parse package name
        let package = if specifier.starts_with('@') {
            if parts.len() < 2 {
                return None;
            }

            format!("{}/{}", parts.remove(0), parts.remove(0))
        } else {
            parts.remove(0).to_string()
        };

        // parse package export key
        let export = if parts.is_empty() {
            ".".to_string()
        } else {
            format!("./{}", parts.join("/"))
        };

        Some(Self { package, export })
    }
}
