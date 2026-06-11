use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Warnings during the check phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Check)]
pub enum CheckWarning {
    /// Code follows an expression that always transfers control.
    ///
    /// ```ds
    /// return value;
    /// process();
    /// ```
    #[diagnostic(code = "WC100", message = "unreachable code")]
    UnreachableCode {
        /// Report the first unreachable expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern can never match after earlier patterns.
    ///
    /// ```ds
    /// match (value) {
    ///     _ => 1,
    ///     true => 2,
    /// }
    /// ```
    #[diagnostic(code = "WC101", message = "unreachable pattern")]
    UnreachablePattern {
        /// Report the unreachable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Extension overload can never win against an earlier declaration.
    ///
    /// ```ds
    /// extension of User {
    ///     show(): string {}
    ///     show(): string {}
    /// }
    /// ```
    #[diagnostic(code = "WC102", message = "overload '{key}' can never be selected")]
    UnreachableOverload {
        /// Report the unreachable overload.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The shadowed member key.
        key: String,
    },

    /// Implementation pairs a foreign contract with a foreign type.
    ///
    /// ```ds
    /// extension of ForeignType implements ForeignContract {}
    /// ```
    #[diagnostic(
        code = "WC103",
        message = "implementing foreign contract '{contract}' for foreign type '{ty}' risks program-wide conflicts"
    )]
    ForeignImplementation {
        /// Report the implementation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The implemented contract.
        contract: String,
        /// The implementing type.
        ty: String,
    },

    /// Cast does not change the expression's type.
    ///
    /// ```ds
    /// const value = 1 as int32 as int32;
    /// ```
    #[diagnostic(code = "WC104", message = "cast to '{ty}' has no effect")]
    RedundantCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The cast target type.
        ty: String,
    },

    /// Runtime condition is statically known.
    ///
    /// ```ds
    /// if (true) {}
    /// ```
    #[diagnostic(code = "WC105", message = "condition is always {value}")]
    ConstantCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The statically known value.
        value: bool,
    },
}
