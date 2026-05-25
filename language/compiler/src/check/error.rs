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
    /// Solver could not determine a required type or static value.
    #[diagnostic(code = "EC100", message = "cannot solve constraints")]
    CannotSolve {
        /// Report the node that requires the solution.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A declaration requires an explicit or contextual type annotation.
    #[diagnostic(code = "EC101", message = "missing type annotation")]
    MissingTypeAnnotation {
        /// Report the declaration that needs a type.
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

    /// Assignment writes through a target that is not writable.
    #[diagnostic(code = "EC204", message = "assignment target is not writable")]
    NotWritable {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Fresh object literal contains a property that the target cannot accept.
    #[diagnostic(code = "EC205", message = "excess property '{key}'")]
    ExcessProperty {
        /// Report the extra property.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The extra property key.
        key: String,
    },

    /// Type cannot be explicitly cast to the requested target type.
    #[diagnostic(code = "EC206", message = "type cannot be cast")]
    InvalidCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Intrinsic marker type appears outside a compiler-recognized language item.
    #[diagnostic(code = "EC207", message = "intrinsic type is not valid here")]
    InvalidIntrinsicType {
        /// Report the intrinsic type expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Const assertion marker appears outside an `as const` expression.
    #[diagnostic(code = "EC208", message = "const type is not valid here")]
    InvalidConstType {
        /// Report the const type expression.
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

    /// No operator overload matches the supplied operands.
    #[diagnostic(code = "EC306", message = "no matching operator '{operator}'")]
    NoMatchingOperator {
        /// Report the operator expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected operator.
        operator: String,
    },

    /// Strict equality operands do not have identity-compatible types.
    #[diagnostic(
        code = "EC307",
        message = "strict equality requires identity-compatible operands"
    )]
    InvalidStrictEquality {
        /// Report the strict equality expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 4xx: expressions
    // -------------------------------------------------------------------------
    /// Condition does not have boolean type.
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
    /// Type is not concrete and therefore has no layout.
    #[diagnostic(code = "EC500", message = "type has no concrete layout")]
    LayoutNotConcrete {
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
