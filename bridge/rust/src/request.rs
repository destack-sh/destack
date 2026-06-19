use destack_bridge_language as bridge;
use destack_source as source;

use crate::{Error, Module, Result};

/// A document accepted by the Rust formatter bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Document {
    /// Loaded module source.
    Module {
        /// Loaded module.
        module: Module,
    },
    /// Ad hoc source text.
    Text {
        /// Display path.
        path: String,
        /// Source text.
        text: String,
    },
}

/// Request accepted by the Rust formatter bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatRequest {
    /// Document to format.
    pub document: Document,
}

/// A lint scope accepted by the Rust linter bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// One module profile.
    Module {
        /// Loaded module.
        module: Module,
        /// Semantic profile id.
        profile: source::ProfileId,
    },
    /// One package.
    Package {
        /// Source package id.
        package: source::PackageId,
    },
    /// Whole workspace.
    Workspace,
}

/// Request accepted by the Rust linter bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintRequest {
    /// Scope to lint.
    pub scope: Scope,
}

/// Request accepted by the Rust build bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildRequest {
    /// Build one module output.
    Module {
        /// Loaded module.
        module: Module,
        /// Target id.
        target: source::TargetId,
        /// Module output kind.
        output: bridge::ModuleBuildKind,
    },
    /// Build one target build payload.
    Build {
        /// Target id.
        target: source::TargetId,
    },
    /// Build one canonical target output.
    Target {
        /// Target id.
        target: source::TargetId,
    },
    /// Build one product output.
    Product {
        /// Product id.
        product: source::ProductId,
    },
}

impl Document {
    /// Create one module document.
    pub fn module(module: Module) -> Self {
        Self::Module { module }
    }

    /// Create one text document.
    pub fn text(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text {
            path: path.into(),
            text: text.into(),
        }
    }
}

impl TryFrom<bridge::Document> for Document {
    type Error = Error;

    /// Convert one bridge document into one Rust document.
    fn try_from(document: bridge::Document) -> Result<Self> {
        match document {
            bridge::Document::Module { module } => {
                let module = Module::try_from(module)?;

                Ok(Self::Module { module })
            }
            bridge::Document::Text { path, text } => Ok(Self::Text { path, text }),
        }
    }
}

impl FormatRequest {
    /// Create one format request.
    pub fn new(document: Document) -> Self {
        Self { document }
    }
}

impl TryFrom<bridge::FormatRequest> for FormatRequest {
    type Error = Error;

    /// Convert one bridge format request into one Rust format request.
    fn try_from(request: bridge::FormatRequest) -> Result<Self> {
        let document = Document::try_from(request.document)?;

        Ok(Self { document })
    }
}

impl Scope {
    /// Create one module lint scope.
    pub fn module(module: Module, profile: source::ProfileId) -> Self {
        Self::Module { module, profile }
    }

    /// Create one package lint scope.
    pub fn package(package: source::PackageId) -> Self {
        Self::Package { package }
    }

    /// Create one workspace lint scope.
    pub fn workspace() -> Self {
        Self::Workspace
    }
}

impl TryFrom<bridge::Scope> for Scope {
    type Error = Error;

    /// Convert one bridge lint scope into one Rust lint scope.
    fn try_from(scope: bridge::Scope) -> Result<Self> {
        match scope {
            bridge::Scope::Module { module, profile } => {
                let module = Module::try_from(module)?;
                let profile = profile.into_source().map_err(Error::new)?;

                Ok(Self::Module { module, profile })
            }
            bridge::Scope::Package { package } => {
                let package = package.into_source().map_err(Error::new)?;

                Ok(Self::Package { package })
            }
            bridge::Scope::Workspace => Ok(Self::Workspace),
        }
    }
}

impl LintRequest {
    /// Create one lint request.
    pub fn new(scope: Scope) -> Self {
        Self { scope }
    }
}

impl TryFrom<bridge::LintRequest> for LintRequest {
    type Error = Error;

    /// Convert one bridge lint request into one Rust lint request.
    fn try_from(request: bridge::LintRequest) -> Result<Self> {
        let scope = Scope::try_from(request.scope)?;

        Ok(Self { scope })
    }
}

impl BuildRequest {
    /// Create one module build request.
    pub fn module(
        module: Module,
        target: source::TargetId,
        output: bridge::ModuleBuildKind,
    ) -> Self {
        Self::Module {
            module,
            target,
            output,
        }
    }

    /// Create one build artifact request.
    pub fn build(target: source::TargetId) -> Self {
        Self::Build { target }
    }

    /// Create one target build request.
    pub fn target(target: source::TargetId) -> Self {
        Self::Target { target }
    }

    /// Create one product build request.
    pub fn product(product: source::ProductId) -> Self {
        Self::Product { product }
    }
}

impl TryFrom<bridge::BuildRequest> for BuildRequest {
    type Error = Error;

    /// Convert one bridge build request into one Rust build request.
    fn try_from(request: bridge::BuildRequest) -> Result<Self> {
        match request {
            bridge::BuildRequest::Module {
                module,
                target,
                output,
            } => {
                let module = Module::try_from(module)?;
                let target = target.into_source().map_err(Error::new)?;

                Ok(Self::Module {
                    module,
                    target,
                    output,
                })
            }
            bridge::BuildRequest::Build { target } => {
                let target = target.into_source().map_err(Error::new)?;

                Ok(Self::Build { target })
            }
            bridge::BuildRequest::Target { target } => {
                let target = target.into_source().map_err(Error::new)?;

                Ok(Self::Target { target })
            }
            bridge::BuildRequest::Product { product } => {
                let product = product.into_source().map_err(Error::new)?;

                Ok(Self::Product { product })
            }
        }
    }
}
