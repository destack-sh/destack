use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Warnings during the check phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Check)]
pub enum CheckWarning {
    // -------------------------------------------------------------------------
    // 1xx: inference
    // -------------------------------------------------------------------------
    /// Authored type constituent adds nothing to its type.
    #[diagnostic(code = "WC100", message = "redundant type constituent")]
    RedundantTypeConstituent {
        /// Report the redundant constituent.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Explicit type argument equals the inferred argument.
    #[diagnostic(code = "WC101", message = "unnecessary type argument")]
    UnnecessaryTypeArgument {
        /// Report the unnecessary argument.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 2xx: relations
    // -------------------------------------------------------------------------
    /// Cast does not change the expression's type.
    ///
    /// ```ds
    /// const value = 1 as int32 as int32;
    /// ```
    #[diagnostic(code = "WC200", message = "cast to '{ty}' has no effect")]
    RedundantCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The cast target type.
        ty: String,
    },

    /// Type assertion does not change the proven type.
    #[diagnostic(code = "WC201", message = "unnecessary type assertion")]
    UnnecessaryTypeAssertion {
        /// Report the unnecessary assertion.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Literal conversion loses information.
    #[diagnostic(code = "WC202", message = "numeric conversion loses precision")]
    LossOfPrecision {
        /// Report the lossy conversion.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 3xx: selection
    // -------------------------------------------------------------------------
    /// Extension overload can never win against an earlier declaration.
    ///
    /// ```ds
    /// extension of User {
    ///     show(): string {}
    ///     show(): string {}
    /// }
    /// ```
    #[diagnostic(code = "WC300", message = "overload '{key}' can never be selected")]
    UnreachableOverload {
        /// Report the unreachable overload.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The shadowed member key.
        key: String,
    },

    /// Resolved definition is deprecated.
    #[diagnostic(code = "WC301", message = "use of deprecated definition")]
    Deprecated {
        /// Report the deprecated use.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Name resolves identically without its qualifier.
    #[diagnostic(code = "WC302", message = "unnecessary qualifier")]
    UnnecessaryQualifier {
        /// Report the unnecessary qualifier.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Diagnostic decorator references no accepted compiler or linter diagnostic.
    #[diagnostic(code = "WC303", message = "unknown diagnostic selector")]
    UnknownDiagnosticSelector {
        /// Report the unknown decorator argument.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 4xx: expressions
    // -------------------------------------------------------------------------
    /// Code follows an expression that always transfers control.
    ///
    /// ```ds
    /// function run(): void {
    ///     return;
    ///     process();
    /// }
    /// ```
    #[diagnostic(code = "WC400", message = "unreachable code")]
    UnreachableCode {
        /// Report the first unreachable expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern coverage proves this pattern can never match.
    #[diagnostic(code = "WC401", message = "unreachable pattern")]
    UnreachablePattern {
        /// Report the unreachable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Runtime condition is statically known.
    ///
    /// ```ds
    /// if (true) {}
    /// ```
    #[diagnostic(code = "WC402", message = "condition is always {value}")]
    ConstantCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The statically known value.
        value: bool,
    },

    /// Assigned value is never observed.
    #[diagnostic(code = "WC403", message = "unused assignment")]
    UnusedAssignment {
        /// Report the unused assignment.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A value marked as requiring use is discarded.
    #[diagnostic(code = "WC404", message = "unused must-use value")]
    UnusedMustUse {
        /// Report the discarded value.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Label is never targeted.
    #[diagnostic(code = "WC405", message = "unused label")]
    UnusedLabel {
        /// Report the unused label.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Checked type and flow prove the condition outcome.
    #[diagnostic(code = "WC406", message = "unnecessary condition")]
    UnnecessaryCondition {
        /// Report the unnecessary condition.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Callback result is discarded despite requiring observation.
    #[diagnostic(code = "WC407", message = "discarded callback result")]
    DiscardedCallbackResult {
        /// Report the discarded result.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Binary expression has a statically fixed result.
    #[diagnostic(code = "WC408", message = "constant binary expression")]
    ConstantBinaryExpression {
        /// Report the constant expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Control transfer from finally suppresses an earlier exit.
    #[diagnostic(code = "WC409", message = "control transfer from finally")]
    UnsafeFinallyControlTransfer {
        /// Report the control transfer.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Switch arm falls through without stating that intent.
    #[diagnostic(code = "WC410", message = "implicit switch fallthrough")]
    Fallthrough {
        /// Report the falling-through arm.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Generator body cannot yield.
    #[diagnostic(code = "WC411", message = "generator has no yield")]
    GeneratorWithoutYield {
        /// Report the generator declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 6xx: declarations
    // -------------------------------------------------------------------------
    /// Implementation pairs a foreign interface with a foreign type.
    ///
    /// ```ds
    /// extension of ForeignType implements ForeignInterface {}
    /// ```
    #[diagnostic(
        code = "WC600",
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
    #[diagnostic(code = "WC601", message = "unused import")]
    UnusedImport {
        /// Report the unused import.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Resolved binding has no attributed use.
    #[diagnostic(code = "WC602", message = "unused binding")]
    UnusedBinding {
        /// Report the unused binding.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Resolved parameter has no attributed use.
    #[diagnostic(code = "WC603", message = "unused parameter")]
    UnusedParameter {
        /// Report the unused parameter.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Private member has no reachable use.
    #[diagnostic(code = "WC604", message = "unused private member")]
    UnusedPrivateMember {
        /// Report the unused member.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },
}
