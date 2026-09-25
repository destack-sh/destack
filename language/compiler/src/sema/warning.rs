use tspp_artifact_macros::Diagnostic;
use tspp_source::ModuleId;

use crate::DiagnosticAnchor;

/// Warnings during the sema phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Check, controllable)]
pub enum CheckWarning {
    // -------------------------------------------------------------------------
    // inference
    // -------------------------------------------------------------------------
    /// Authored type constituent adds nothing to its type.
    #[diagnostic(
        id = "redundant-type-constituent",
        message = "redundant type constituent"
    )]
    RedundantTypeConstituent {
        /// Report the redundant constituent.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Explicit type argument equals the inferred argument.
    #[diagnostic(
        id = "unnecessary-type-argument",
        message = "unnecessary type argument"
    )]
    UnnecessaryTypeArgument {
        /// Report the unnecessary argument.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // relations
    // -------------------------------------------------------------------------
    /// Cast does not change the expression's type.
    ///
    /// ```tspp
    /// const value = 1 as int32 as int32;
    /// ```
    #[diagnostic(id = "redundant-cast", message = "cast to '{ty}' has no effect")]
    RedundantCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The cast target type.
        ty: String,
    },

    /// Type assertion does not change the proven type.
    #[diagnostic(
        id = "unnecessary-type-assertion",
        message = "unnecessary type assertion"
    )]
    UnnecessaryTypeAssertion {
        /// Report the unnecessary assertion.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Literal conversion loses information.
    #[diagnostic(
        id = "loss-of-precision",
        message = "numeric conversion loses precision"
    )]
    LossOfPrecision {
        /// Report the lossy conversion.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Constant shift amount reaches past the shifted width.
    ///
    /// ```tspp
    /// declare const bits: int32;
    /// const spilled = bits << 32;
    /// ```
    #[diagnostic(
        id = "shift-out-of-range",
        message = "shift amount {amount} is out of range for '{ty}'"
    )]
    ShiftOutOfRange {
        /// Report the shift expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The constant shift amount.
        amount: i64,
        /// The shifted integer type.
        ty: String,
    },

    // -------------------------------------------------------------------------
    // selection
    // -------------------------------------------------------------------------
    /// Extension overload can never win against an earlier declaration.
    ///
    /// ```tspp
    /// extension of User {
    ///     show(): string {}
    ///     show(): string {}
    /// }
    /// ```
    #[diagnostic(
        id = "unreachable-overload",
        message = "overload '{key}' can never be selected"
    )]
    UnreachableOverload {
        /// Report the unreachable overload.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The shadowed member key.
        key: String,
    },

    /// Resolved definition is deprecated.
    #[diagnostic(id = "deprecated", message = "use of deprecated definition")]
    Deprecated {
        /// Report the deprecated use.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Resolved definition is experimental.
    #[diagnostic(id = "experimental", message = "use of experimental definition")]
    Experimental {
        /// Report the experimental use.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Name resolves identically without its qualifier.
    #[diagnostic(id = "unnecessary-qualifier", message = "unnecessary qualifier")]
    UnnecessaryQualifier {
        /// Report the unnecessary qualifier.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // expressions
    // -------------------------------------------------------------------------
    /// Code follows an expression that always transfers control.
    ///
    /// ```tspp
    /// function run(): void {
    ///     return;
    ///     process();
    /// }
    /// ```
    #[diagnostic(id = "unreachable-code", message = "unreachable code")]
    UnreachableCode {
        /// Report the first unreachable expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern coverage proves this pattern can never match.
    #[diagnostic(id = "unreachable-pattern", message = "unreachable pattern")]
    UnreachablePattern {
        /// Report the unreachable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Runtime condition is statically known.
    ///
    /// ```tspp
    /// if (true) {}
    /// ```
    #[diagnostic(id = "constant-condition", message = "condition is always {value}")]
    ConstantCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The statically known value.
        value: bool,
    },

    /// Assigned value is never observed.
    #[diagnostic(id = "unused-assignment", message = "unused assignment")]
    UnusedAssignment {
        /// Report the unused assignment.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A value marked as requiring use is discarded.
    #[diagnostic(id = "unused-must-use", message = "unused must-use value")]
    UnusedMustUse {
        /// Report the discarded value.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Label is never targeted.
    #[diagnostic(id = "unused-label", message = "unused label")]
    UnusedLabel {
        /// Report the unused label.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Checked type and flow prove the condition outcome.
    #[diagnostic(id = "unnecessary-condition", message = "unnecessary condition")]
    UnnecessaryCondition {
        /// Report the unnecessary condition.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Callback result is discarded despite requiring observation.
    #[diagnostic(
        id = "discarded-callback-result",
        message = "discarded callback result"
    )]
    DiscardedCallbackResult {
        /// Report the discarded result.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Binary expression has a statically fixed result.
    #[diagnostic(
        id = "constant-binary-expression",
        message = "constant binary expression"
    )]
    ConstantBinaryExpression {
        /// Report the constant expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Control transfer from finally suppresses an earlier exit.
    #[diagnostic(
        id = "unsafe-finally-control-transfer",
        message = "control transfer from finally"
    )]
    UnsafeFinallyControlTransfer {
        /// Report the control transfer.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Switch arm falls through without stating that intent.
    #[diagnostic(id = "fallthrough", message = "implicit switch fallthrough")]
    Fallthrough {
        /// Report the falling-through arm.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Generator body cannot yield.
    #[diagnostic(id = "generator-without-yield", message = "generator has no yield")]
    GeneratorWithoutYield {
        /// Report the generator declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // declarations
    // -------------------------------------------------------------------------
    /// Implementation pairs a foreign interface with a foreign type.
    ///
    /// ```tspp
    /// extension of ForeignType implements ForeignInterface {}
    /// ```
    #[diagnostic(
        id = "non-local-implementation",
        message = "implementation of foreign interface '{interface}' for foreign type '{ty}' is not local to this package"
    )]
    NonLocalImplementation {
        /// Report the implementation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The implemented interface.
        interface: String,
        /// The implementing type.
        ty: String,
    },

    /// Resolved import has no attributed use.
    #[diagnostic(id = "unused-import", message = "unused import")]
    UnusedImport {
        /// Report the unused import.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Resolved binding has no attributed use.
    #[diagnostic(id = "unused-binding", message = "unused binding")]
    UnusedBinding {
        /// Report the unused binding.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Resolved parameter has no attributed use.
    #[diagnostic(id = "unused-parameter", message = "unused parameter")]
    UnusedParameter {
        /// Report the unused parameter.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Private member has no reachable use.
    #[diagnostic(id = "unused-private-member", message = "unused private member")]
    UnusedPrivateMember {
        /// Report the unused member.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },
}
