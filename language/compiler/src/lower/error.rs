use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_dir as dir;
use destack_source::ModuleId;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Lower)]
pub enum LowerError {
    // -------------------------------------------------------------------------
    // 1xx: Type issues
    // -------------------------------------------------------------------------
    /// Unsupported type.
    #[diagnostic(code = "EL100", message = "unsupported type: {message}")]
    UnsupportedType {
        /// Report the source that introduced the unsupported type.
        anchor: DiagnosticAnchor,
        /// Identify the unsupported type id.
        ty: dir::GlobalTypeId,
        /// Describe why the type is unsupported.
        message: String,
    },

    /// Missing type.
    #[diagnostic(code = "EL101", message = "missing type")]
    MissingType {
        /// Report the source that lacks type information.
        anchor: DiagnosticAnchor,
    },

    /// Non-boolean condition in control flow.
    #[diagnostic(code = "EL102", message = "condition requires boolean type")]
    NonBooleanCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// Context where boolean was expected (if, while, for, etc.).
        context: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Construct issues
    // -------------------------------------------------------------------------
    /// Unsupported node (generic catch-all for constructs not yet implemented).
    #[diagnostic(code = "EL200", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        /// Report the construct that cannot be lowered.
        anchor: DiagnosticAnchor,
        /// Describe why the construct is unsupported.
        message: String,
    },

    /// Static arguments are not supported.
    #[diagnostic(code = "EL201", message = "static type arguments not supported")]
    StaticArgumentsNotSupported {
        /// Report the call or member access node.
        anchor: DiagnosticAnchor,
    },

    /// Unsupported binding pattern.
    #[diagnostic(code = "EL202", message = "unsupported binding pattern")]
    UnsupportedPattern {
        /// Report the pattern node.
        anchor: DiagnosticAnchor,
    },

    /// Invalid static argument.
    #[diagnostic(code = "EL203", message = "invalid static argument: {message}")]
    InvalidStaticArgument {
        /// Report the source that carries the static arguments.
        anchor: DiagnosticAnchor,
        /// Describe why the static argument is invalid.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 3xx: Symbol resolution issues
    // -------------------------------------------------------------------------
    /// Unresolved symbol reference.
    #[diagnostic(code = "EL300", message = "unresolved symbol")]
    UnresolvedSymbol {
        /// Report the reference expression.
        anchor: DiagnosticAnchor,
    },

    /// Missing function for symbol.
    #[diagnostic(code = "EL301", message = "missing function for symbol")]
    MissingFunction {
        /// Report the call node.
        anchor: DiagnosticAnchor,
        /// The symbol that should reference a function.
        symbol: dir::GlobalSymbolId,
    },

    /// Missing Resolution for method call.
    #[diagnostic(code = "EL302", message = "method call missing Resolution")]
    MissingResolution {
        /// Report the method call node.
        anchor: DiagnosticAnchor,
    },

    /// `this` reference outside of method context.
    #[diagnostic(code = "EL303", message = "`this` reference outside of method context")]
    ThisOutsideMethod {
        /// Report the `this` expression.
        anchor: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 4xx: Struct/aggregate issues
    // -------------------------------------------------------------------------
    /// Missing struct layout.
    #[diagnostic(code = "EL400", message = "missing struct layout")]
    MissingLayout {
        /// Report the struct expression.
        anchor: DiagnosticAnchor,
    },

    /// Missing field initializer.
    #[diagnostic(code = "EL401", message = "struct field {field_index} not initialized")]
    MissingFieldInitializer {
        /// Report the struct literal.
        anchor: DiagnosticAnchor,
        /// Index of the uninitialized field.
        field_index: u32,
    },

    /// Duplicate field initializer.
    #[diagnostic(code = "EL402", message = "duplicate struct field initializer")]
    DuplicateField {
        /// Report the duplicate field property.
        anchor: DiagnosticAnchor,
    },

    /// Field not found in struct type.
    #[diagnostic(code = "EL403", message = "field not found in type")]
    FieldNotFound {
        /// Report the member expression.
        anchor: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 5xx: Control flow issues
    // -------------------------------------------------------------------------
    /// Missing terminator (function body doesn't return).
    #[diagnostic(code = "EL500", message = "function body missing terminator")]
    MissingTerminator {
        /// Report the function declaration.
        anchor: DiagnosticAnchor,
    },

    /// Break/continue outside of loop.
    #[diagnostic(code = "EL501", message = "break/continue outside of loop")]
    BreakContinueOutsideLoop {
        /// Report the break/continue expression.
        anchor: DiagnosticAnchor,
    },

    /// Conditional expression missing else branch (value required).
    #[diagnostic(
        code = "EL502",
        message = "conditional expression requires else branch"
    )]
    MissingElseBranch {
        /// Report the if expression.
        anchor: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 6xx: Operator/cast issues
    // -------------------------------------------------------------------------
    /// Unsupported binary operator.
    #[diagnostic(code = "EL600", message = "unsupported binary operator")]
    UnsupportedBinaryOperator {
        /// Report the binary expression.
        anchor: DiagnosticAnchor,
    },

    /// Unsupported unary operator.
    #[diagnostic(code = "EL601", message = "unsupported unary operator")]
    UnsupportedUnaryOperator {
        /// Report the unary expression.
        anchor: DiagnosticAnchor,
    },

    /// Unsupported cast.
    #[diagnostic(code = "EL602", message = "unsupported cast")]
    UnsupportedCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal lowering error.
    #[diagnostic(code = "EL900", message = "internal error: {message}")]
    Internal {
        /// Anchor the error to a module.
        anchor: DiagnosticAnchor,
        /// Identify the module where lowering failed.
        module: ModuleId,
        /// Describe the internal failure.
        message: String,
    },
}
