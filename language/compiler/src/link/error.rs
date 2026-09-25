use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;
use tspp_source::{PackageId, ProductId, TargetId};

/// Errors during the link phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Link)]
pub enum LinkError {
    // -------------------------------------------------------------------------
    // targets
    // -------------------------------------------------------------------------
    /// Missing target.
    #[diagnostic(id = "missing-link-target", message = "missing target: {target}")]
    MissingTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
    },

    /// Invalid target configuration.
    #[diagnostic(
        id = "invalid-link-target",
        message = "invalid target: {target}: {message}"
    )]
    InvalidTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Invalid module kind for one linked subject.
    #[diagnostic(
        id = "invalid-module-kind",
        message = "invalid module kind: {target}: {subject} expected {expected}, found '{found}'"
    )]
    InvalidModuleKind {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        subject: String,
        expected: String,
        found: String,
    },

    /// Unsupported suffix on one linked reference.
    #[diagnostic(
        id = "unsupported-reference-suffix",
        message = "unsupported reference suffix: {target}: {reference} does not support query or fragment suffix yet: '{value}'"
    )]
    UnsupportedReferenceSuffix {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        reference: String,
        value: String,
    },

    /// Invalid output path state for one linked subject.
    #[diagnostic(
        id = "invalid-output-path",
        message = "invalid output path: {target}: {subject} has no usable emitted path segment from '{value}'"
    )]
    InvalidOutputPath {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        subject: String,
        value: String,
    },

    /// Missing product.
    #[diagnostic(id = "missing-product", message = "missing product: {product}")]
    MissingProduct {
        anchor: DiagnosticAnchor,
        package: PackageId,
        product: ProductId,
    },

    /// Invalid product configuration.
    #[diagnostic(
        id = "invalid-product",
        message = "invalid product: {product}: {message}"
    )]
    InvalidProduct {
        anchor: DiagnosticAnchor,
        package: PackageId,
        product: ProductId,
        message: String,
    },

    /// Invalid executable input.
    #[diagnostic(id = "invalid-executable-input", message = "invalid input: {context}")]
    InvalidInput {
        /// The package containing the invalid input.
        anchor: DiagnosticAnchor,
        /// The package being linked.
        package: PackageId,
        /// The invalid input detail.
        context: String,
    },

    /// Type mismatch while linking a Program.
    #[diagnostic(
        id = "type-mismatch",
        message = "type mismatch: expected {expected}, found {actual}"
    )]
    TypeMismatch {
        /// The package containing the mismatched type.
        anchor: DiagnosticAnchor,
        /// The package being linked.
        package: PackageId,
        /// The expected type.
        expected: String,
        /// The actual type.
        actual: String,
    },

    /// Unsupported zero initializer while linking executable output.
    #[diagnostic(
        id = "unsupported-zero-initializer",
        message = "unsupported zero initializer: {ty}"
    )]
    UnsupportedZeroInitializer {
        /// The package containing the unsupported initializer.
        anchor: DiagnosticAnchor,
        /// The package being linked.
        package: PackageId,
        /// The unsupported initialized type.
        ty: String,
    },

    /// Executable layout cannot encode a value.
    #[diagnostic(id = "layout-overflow", message = "layout overflow: {context}")]
    LayoutOverflow {
        /// The package containing the overflowing layout.
        anchor: DiagnosticAnchor,
        /// The package being linked.
        package: PackageId,
        /// The layout overflow detail.
        context: String,
    },

    // -------------------------------------------------------------------------
    // internal failures
    // -------------------------------------------------------------------------
    /// Internal error during linking.
    #[diagnostic(id = "internal-link-error", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        package: PackageId,
        message: String,
    },
}

impl LinkError {
    /// Create one invalid Program input error.
    pub(crate) fn invalid_input(package: PackageId, context: impl Into<String>) -> Self {
        Self::InvalidInput {
            anchor: package.into(),
            package,
            context: context.into(),
        }
    }
}
