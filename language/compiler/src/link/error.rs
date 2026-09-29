use crate::{CompilerError, DiagnosticAnchor};
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
}

/// Return one internal error for invalid Program input.
pub(crate) fn invalid_program_input(context: impl Into<String>) -> CompilerError {
    CompilerError::Internal {
        message: format!("invalid program input: {}", context.into()),
    }
}

/// Return one internal error for a type the linked Program disagrees on.
pub(crate) fn type_mismatch(
    expected: impl Into<String>,
    actual: impl Into<String>,
) -> CompilerError {
    CompilerError::Internal {
        message: format!(
            "type mismatch: expected {}, found {}",
            expected.into(),
            actual.into()
        ),
    }
}

/// Return one internal error for a zero initializer the linker cannot encode.
pub(crate) fn unsupported_zero_initializer(ty: impl Into<String>) -> CompilerError {
    CompilerError::Internal {
        message: format!("unsupported zero initializer: {}", ty.into()),
    }
}

/// Return one internal error for a layout the executable cannot encode.
pub(crate) fn layout_overflow(context: impl Into<String>) -> CompilerError {
    CompilerError::Internal {
        message: format!("layout overflow: {}", context.into()),
    }
}
