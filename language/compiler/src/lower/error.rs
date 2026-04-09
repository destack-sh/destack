use crate::{
    CompileError, DiagnosticAnchor, DiagnosticDefinition, RequirementError, RequirementSet,
};
use destack_compiler_macros::DefineError;
use destack_core::StringId;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::Repository;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Lower)]
pub enum LowerError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EM000", r#yield)]
    Yield {
        /// Carry the requirement that must be satisfied before lowering can proceed.
        requirement: RequirementSet,
    },

    /// Yield requirement has failed.
    #[error(code = "EM001", yield_failed)]
    UnsatisfiedRequirement {
        /// Carry the requirement that failed to resolve.
        requirement: RequirementSet,
    },

    /// Task was skipped due to stale versions.
    #[error(code = "EM002", message = "task skipped")]
    Skipped,

    // -------------------------------------------------------------------------
    // 1xx: Type issues
    // -------------------------------------------------------------------------
    /// Unsupported type.
    #[error(code = "EM100", message = "unsupported type {ty}: {message}")]
    UnsupportedType {
        /// Report the node that introduced the unsupported type.
        node: dir::AnchoredGlobalNodeId,
        /// Identify the unsupported type id.
        ty: dir::GlobalTypeId,
        /// Describe why the type is unsupported.
        message: String,
    },

    /// Missing type.
    #[error(code = "EM101", message = "missing type")]
    MissingType {
        /// Report the node that lacks type information.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Non-boolean condition in control flow.
    #[error(code = "EM102", message = "condition requires boolean type")]
    NonBooleanCondition {
        /// Report the condition expression node.
        node: dir::AnchoredGlobalNodeId,
        /// Context where boolean was expected (if, while, for, etc.).
        context: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: Construct issues
    // -------------------------------------------------------------------------
    /// Unsupported node (generic catch-all for constructs not yet implemented).
    #[error(code = "EM200", message = "unsupported construct: {message}")]
    UnsupportedConstruct {
        /// Report the offending node that cannot be lowered.
        node: dir::AnchoredGlobalNodeId,
        /// Describe why the construct is unsupported.
        message: String,
    },

    /// Static arguments are not supported.
    #[error(code = "EM201", message = "static type arguments not supported")]
    StaticArgumentsNotSupported {
        /// Report the call or member access node.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Unsupported binding pattern.
    #[error(code = "EM202", message = "unsupported binding pattern")]
    UnsupportedPattern {
        /// Report the pattern node.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Invalid static argument.
    #[error(code = "EM203", message = "invalid static argument: {message}")]
    InvalidStaticArgument {
        /// Report the node that carries the static arguments.
        node: dir::AnchoredGlobalNodeId,
        /// Describe why the static argument is invalid.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 3xx: Symbol resolution issues
    // -------------------------------------------------------------------------
    /// Unresolved symbol reference.
    #[error(code = "EM300", message = "unresolved symbol")]
    UnresolvedSymbol {
        /// Report the reference expression.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Missing function for symbol.
    #[error(code = "EM301", message = "missing function for symbol {symbol}")]
    MissingFunction {
        /// Report the call node.
        node: dir::AnchoredGlobalNodeId,
        /// The symbol that should reference a function.
        symbol: dir::GlobalSymbolId,
    },

    /// Missing Resolution for method call.
    #[error(code = "EM302", message = "method call missing Resolution")]
    MissingResolution {
        /// Report the method call node.
        node: dir::AnchoredGlobalNodeId,
        /// Method name (for diagnostic context).
        method_name: StringId,
    },

    /// `this` reference outside of method context.
    #[error(code = "EM303", message = "`this` reference outside of method context")]
    ThisOutsideMethod {
        /// Report the `this` expression.
        node: dir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 4xx: Struct/aggregate issues
    // -------------------------------------------------------------------------
    /// Missing struct layout.
    #[error(code = "EM400", message = "missing struct layout")]
    MissingLayout {
        /// Report the struct expression.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Missing field initializer.
    #[error(code = "EM401", message = "struct field {field_index} not initialized")]
    MissingFieldInitializer {
        /// Report the struct literal.
        node: dir::AnchoredGlobalNodeId,
        /// Index of the uninitialized field.
        field_index: u32,
    },

    /// Duplicate field initializer.
    #[error(code = "EM402", message = "duplicate struct field initializer")]
    DuplicateField {
        /// Report the duplicate field property.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Field not found in struct type.
    #[error(code = "EM403", message = "field not found in type")]
    FieldNotFound {
        /// Report the member expression.
        node: dir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 5xx: Control flow issues
    // -------------------------------------------------------------------------
    /// Missing terminator (function body doesn't return).
    #[error(code = "EM500", message = "function body missing terminator")]
    MissingTerminator {
        /// Report the function declaration.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Break/continue outside of loop.
    #[error(code = "EM501", message = "break/continue outside of loop")]
    BreakContinueOutsideLoop {
        /// Report the break/continue expression.
        node: dir::AnchoredGlobalNodeId,
        /// Label if present.
        label: Option<StringId>,
    },

    /// Conditional expression missing else branch (value required).
    #[error(
        code = "EM502",
        message = "conditional expression requires else branch"
    )]
    MissingElseBranch {
        /// Report the if expression.
        node: dir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 6xx: Operator/cast issues
    // -------------------------------------------------------------------------
    /// Unsupported binary operator.
    #[error(code = "EM600", message = "unsupported binary operator")]
    UnsupportedBinaryOperator {
        /// Report the binary expression.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Unsupported unary operator.
    #[error(code = "EM601", message = "unsupported unary operator")]
    UnsupportedUnaryOperator {
        /// Report the unary expression.
        node: dir::AnchoredGlobalNodeId,
    },

    /// Unsupported cast.
    #[error(code = "EM602", message = "unsupported cast")]
    UnsupportedCast {
        /// Report the cast expression.
        node: dir::AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal
    // -------------------------------------------------------------------------
    /// Internal lowering error.
    #[error(code = "EM900", message = "internal error: {message}")]
    Internal {
        /// Anchor the error to a module.
        module: ModuleId,
        /// Describe the internal failure.
        message: String,
    },
}
