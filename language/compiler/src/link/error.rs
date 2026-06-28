use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::{PackageId, ProductId, TargetId};

/// Errors during the link phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Link)]
pub enum LinkError {
    // -------------------------------------------------------------------------
    // 1xx: Target issues
    // -------------------------------------------------------------------------
    /// Missing target.
    #[diagnostic(code = "EK100", message = "missing target: {target}")]
    MissingTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
    },

    /// Invalid target configuration.
    #[diagnostic(code = "EK101", message = "invalid target: {target}: {message}")]
    InvalidTarget {
        anchor: DiagnosticAnchor,
        package: PackageId,
        target: TargetId,
        message: String,
    },

    /// Invalid module kind for one linked subject.
    #[diagnostic(
        code = "EK102",
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
        code = "EK103",
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
        code = "EK104",
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
    #[diagnostic(code = "EK105", message = "missing product: {product}")]
    MissingProduct {
        anchor: DiagnosticAnchor,
        package: PackageId,
        product: ProductId,
    },

    /// Invalid product configuration.
    #[diagnostic(code = "EK106", message = "invalid product: {product}: {message}")]
    InvalidProduct {
        anchor: DiagnosticAnchor,
        package: PackageId,
        product: ProductId,
        message: String,
    },

    /// Invalid executable input.
    #[diagnostic(code = "EK107", message = "invalid input: {context}")]
    InvalidInput {
        anchor: DiagnosticAnchor,
        package: PackageId,
        context: String,
    },

    /// Type mismatch while linking executable output.
    #[diagnostic(
        code = "EK108",
        message = "type mismatch: expected {expected}, found {actual}"
    )]
    TypeMismatch {
        anchor: DiagnosticAnchor,
        package: PackageId,
        expected: String,
        actual: String,
    },

    /// Invalid instruction while linking executable output.
    #[diagnostic(code = "EK109", message = "invalid instruction: {context}")]
    InvalidInstruction {
        anchor: DiagnosticAnchor,
        package: PackageId,
        context: String,
    },

    /// Invalid cast while linking executable output.
    #[diagnostic(code = "EK110", message = "invalid cast: {context}")]
    InvalidCast {
        anchor: DiagnosticAnchor,
        package: PackageId,
        context: String,
    },

    /// Invalid field access while linking executable output.
    #[diagnostic(
        code = "EK111",
        message = "invalid field access: field {index} but value has {field_count} field(s)"
    )]
    InvalidFieldAccess {
        anchor: DiagnosticAnchor,
        package: PackageId,
        index: u32,
        field_count: usize,
    },

    /// Invalid pointer type while linking executable output.
    #[diagnostic(code = "EK112", message = "invalid pointer type: {actual}")]
    InvalidPointerType {
        anchor: DiagnosticAnchor,
        package: PackageId,
        actual: String,
    },

    /// Unsupported instruction while linking executable output.
    #[diagnostic(code = "EK113", message = "unsupported instruction: {name}")]
    UnsupportedInstruction {
        anchor: DiagnosticAnchor,
        package: PackageId,
        name: String,
    },

    /// Unsupported zero initializer while linking executable output.
    #[diagnostic(code = "EK114", message = "unsupported zero initializer: {ty}")]
    UnsupportedZeroInitializer {
        anchor: DiagnosticAnchor,
        package: PackageId,
        ty: String,
    },

    /// Undefined function while linking executable output.
    #[diagnostic(code = "EK115", message = "undefined function: {function}")]
    UndefinedFunction {
        anchor: DiagnosticAnchor,
        package: PackageId,
        function: String,
    },

    /// Executable layout cannot encode a value.
    #[diagnostic(code = "EK116", message = "layout overflow: {context}")]
    LayoutOverflow {
        anchor: DiagnosticAnchor,
        package: PackageId,
        context: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal error during linking.
    #[diagnostic(code = "EK900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        package: PackageId,
        message: String,
    },
}
