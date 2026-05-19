use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the check phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Check)]
pub enum CheckError {
    // -------------------------------------------------------------------------
    // 1xx: inference
    // -------------------------------------------------------------------------
    /// Type inference could not determine a required type.
    #[diagnostic(code = "EC100", message = "cannot infer type")]
    CannotInferType {
        /// Report the node that requires the type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Static evaluation could not determine a required value.
    #[diagnostic(code = "EC101", message = "cannot evaluate static value")]
    CannotEvaluateStatic {
        /// Report the static expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type solving found an illegal recursive type.
    #[diagnostic(code = "EC102", message = "recursive type is not valid here")]
    RecursiveType {
        /// Report the cycle source.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Static evaluation found an illegal recursive value.
    #[diagnostic(code = "EC103", message = "recursive static value is not valid here")]
    RecursiveStatic {
        /// Report the cycle source.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 2xx: relations
    // -------------------------------------------------------------------------
    /// Source type is not assignable to target type.
    #[diagnostic(code = "EC200", message = "type is not assignable")]
    NotAssignable {
        /// Report the assignment source.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type does not satisfy a required constraint.
    #[diagnostic(code = "EC201", message = "type does not satisfy constraint")]
    ConstraintNotSatisfied {
        /// Report the constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type does not extend a required base type.
    #[diagnostic(code = "EC202", message = "type does not extend required type")]
    DoesNotExtend {
        /// Report the extends clause or constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type does not implement a required contract.
    #[diagnostic(code = "EC203", message = "type does not implement required contract")]
    DoesNotImplement {
        /// Report the implements clause or constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Assignment attempts to mutate readonly storage.
    #[diagnostic(code = "EC204", message = "cannot mutate readonly value")]
    MutateReadonly {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 3xx: selection
    // -------------------------------------------------------------------------
    /// Receiver type does not contain a selected member.
    #[diagnostic(code = "EC300", message = "missing member '{key}'")]
    MissingMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
    },

    /// Value is not callable.
    #[diagnostic(code = "EC301", message = "value is not callable")]
    NotCallable {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// No overload matches the supplied arguments.
    #[diagnostic(code = "EC302", message = "no matching call overload")]
    NoMatchingCall {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Member selection has multiple valid targets.
    #[diagnostic(code = "EC303", message = "ambiguous member '{key}'")]
    AmbiguousMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
    },

    /// Call selection has multiple valid targets.
    #[diagnostic(code = "EC304", message = "ambiguous call")]
    AmbiguousCall {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Selected member is not accessible from the current scope.
    #[diagnostic(code = "EC305", message = "member '{key}' is not accessible")]
    InaccessibleMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
    },

    // -------------------------------------------------------------------------
    // 4xx: expressions
    // -------------------------------------------------------------------------
    /// Runtime condition does not have boolean type.
    #[diagnostic(code = "EC400", message = "condition requires boolean type")]
    NonBooleanCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Static condition could not be evaluated to a boolean value.
    #[diagnostic(code = "EC401", message = "static condition requires boolean value")]
    InvalidStaticCondition {
        /// Report the static condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Control flow construct is not valid in its current scope.
    #[diagnostic(code = "EC402", message = "invalid control flow: {message}")]
    InvalidControlFlow {
        /// Report the control flow construct.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// Describe the invalid control flow.
        message: String,
    },

    /// Pattern matching does not cover every possible value.
    #[diagnostic(code = "EC403", message = "pattern match is not exhaustive")]
    NonExhaustivePattern {
        /// Report the match expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern can never match after earlier patterns.
    #[diagnostic(code = "EC404", message = "pattern is unreachable")]
    UnreachablePattern {
        /// Report the unreachable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Local value is used before it is definitely assigned.
    #[diagnostic(code = "EC405", message = "value is used before assignment")]
    UseBeforeAssigned {
        /// Report the use.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 5xx: representation
    // -------------------------------------------------------------------------
    /// Type cannot be represented as a concrete value.
    #[diagnostic(code = "EC500", message = "type has no concrete layout")]
    LayoutNotRealizable {
        /// Report the layout request.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Representation decorator is not valid for the declaration.
    #[diagnostic(code = "EC501", message = "invalid representation: {message}")]
    InvalidRepresentation {
        /// Report the representation decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// Describe why the representation is invalid.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 6xx: declarations
    // -------------------------------------------------------------------------
    /// Override declaration does not match an inherited member.
    #[diagnostic(code = "EC600", message = "invalid override")]
    InvalidOverride {
        /// Report the override declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Concrete type does not implement an abstract member.
    #[diagnostic(code = "EC601", message = "abstract member is not implemented")]
    UnimplementedAbstractMember {
        /// Report the concrete declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The abstract member name.
        member: String,
    },

    /// Abstract type cannot be constructed.
    #[diagnostic(code = "EC602", message = "abstract type cannot be constructed")]
    CannotConstructAbstractType {
        /// Report the constructor call.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 9xx: internal
    // -------------------------------------------------------------------------
    /// Internal check failure.
    #[diagnostic(code = "EC900", message = "internal error: {message}")]
    Internal {
        /// Anchor the error to a module.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// Describe the internal failure.
        message: String,
    },
}
