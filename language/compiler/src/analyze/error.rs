use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_base::StringId;
use destack_compiler_macros::DefineError;
use destack_dir::{
    AnchoredGlobalNodeId, FunctionAbstraction, GlobalSymbolId, GlobalTypeId, StaticKey, Visibility,
};
use destack_workspace::Program;

/// Errors during the analyze phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Analyze)]
pub enum AnalyzeError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / dependency
    // -------------------------------------------------------------------------
    /// Wait for task dependency.
    #[error(code = "EA000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EA001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    // -------------------------------------------------------------------------
    // 1xx: Type errors
    // -------------------------------------------------------------------------
    /// Missing type for an expression.
    #[error(code = "EA100", message = "missing type")]
    MissingType { node: AnchoredGlobalNodeId },

    /// Type is not assignable to the expected type.
    #[error(
        code = "EA101",
        message = "type {actual_ty} is not assignable to type {expected_ty}"
    )]
    UnassignableType {
        node: AnchoredGlobalNodeId,
        expected_ty: GlobalTypeId,
        actual_ty: GlobalTypeId,
    },

    /// Type does not satisfy the expected type (satisfies expression).
    #[error(code = "EA102", message = "expected {expected_ty}, found {actual_ty}")]
    UnsatisfiedType {
        node: AnchoredGlobalNodeId,
        expected_ty: GlobalTypeId,
        actual_ty: GlobalTypeId,
    },

    /// Invalid casts (unsafe or impossible with static rules).
    #[error(code = "EA103", message = "cannot cast type {from_ty} to {to_ty}")]
    InvalidCast {
        node: AnchoredGlobalNodeId,
        from_ty: GlobalTypeId,
        to_ty: GlobalTypeId,
    },

    /// Enum member value has an invalid backing type.
    #[error(code = "EA104", message = "invalid enum backing type {ty}")]
    InvalidEnumBackingType {
        node: AnchoredGlobalNodeId,
        ty: GlobalTypeId,
    },

    /// Strict equality requires identity types.
    #[error(
        code = "EA105",
        message = "strict equality not supported for non-identity type {ty}"
    )]
    InvalidStrictEquality {
        node: AnchoredGlobalNodeId,
        ty: GlobalTypeId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Callable / member / operator errors
    // -------------------------------------------------------------------------
    /// Calling non-callable.
    #[error(code = "EA200", message = "calling non-callable")]
    NonCallable { node: AnchoredGlobalNodeId },

    /// Indexing non-indexable.
    #[error(code = "EA201", message = "indexing non-indexable")]
    NonIndexable { node: AnchoredGlobalNodeId },

    /// Missing member on type.
    #[error(
        code = "EA202",
        message = "property {member_key} does not exist on type {receiver_ty}"
    )]
    MissingMember {
        node: AnchoredGlobalNodeId,
        receiver_ty: GlobalTypeId,
        member_key: StaticKey,
    },

    /// No overload found for operator/method with given types.
    #[error(
        code = "EA203",
        message = "no matching overload for type {receiver_ty}"
    )]
    NoOverload {
        node: AnchoredGlobalNodeId,
        receiver_ty: GlobalTypeId,
    },

    /// Ambiguous overload: multiple candidates match equally well.
    #[error(code = "EA204", message = "ambiguous overload: {candidates}")]
    AmbiguousOverload {
        node: AnchoredGlobalNodeId,
        candidates: Vec<GlobalSymbolId>,
    },

    /// Operator not supported for type.
    #[error(code = "EA205", message = "operator not supported for type {ty}")]
    UnsupportedOperator {
        node: AnchoredGlobalNodeId,
        ty: GlobalTypeId,
    },

    /// Static arguments specified on both the member and the call.
    #[error(
        code = "EA206",
        message = "static arguments specified on both member and call"
    )]
    ConflictingStaticArguments { node: AnchoredGlobalNodeId },

    /// Property access is only available via index signature.
    #[error(
        code = "EA207",
        message = "property {member_key} is only available via index signature"
    )]
    PropertyAccessFromIndexSignature {
        node: AnchoredGlobalNodeId,
        receiver_ty: GlobalTypeId,
        member_key: StaticKey,
    },

    /// Excess property in object literal.
    #[error(
        code = "EA208",
        message = "excess property {member_key} in object literal for type {expected_ty}"
    )]
    ExcessProperty {
        node: AnchoredGlobalNodeId,
        expected_ty: GlobalTypeId,
        member_key: StaticKey,
    },

    /// Invalid import.meta usage.
    #[error(code = "EA209", message = "import.meta is only available in modules")]
    InvalidImportMeta { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 3xx: Control flow
    // -------------------------------------------------------------------------
    /// Invalid break.
    #[error(code = "EA300", message = "invalid break to '{label}'")]
    InvalidBreak {
        node: AnchoredGlobalNodeId,
        label: Option<StringId>,
    },

    /// Break values are not allowed in switch statements.
    #[error(code = "EA308", message = "switch break cannot have a value")]
    InvalidSwitchBreakValue { node: AnchoredGlobalNodeId },

    /// Switch cases must use expression patterns.
    #[error(code = "EA309", message = "switch cases require expression patterns")]
    InvalidSwitchCasePattern { node: AnchoredGlobalNodeId },

    /// Switch cases do not support guards.
    #[error(code = "EA310", message = "switch cases do not support guards")]
    InvalidSwitchCaseGuard { node: AnchoredGlobalNodeId },

    /// Invalid continue.
    #[error(code = "EA301", message = "invalid continue to '{label}'")]
    InvalidContinue {
        node: AnchoredGlobalNodeId,
        label: Option<StringId>,
    },

    /// Invalid await.
    #[error(code = "EA302", message = "invalid await")]
    InvalidAwait { node: AnchoredGlobalNodeId },

    /// Invalid yield.
    #[error(code = "EA303", message = "invalid yield")]
    InvalidYield { node: AnchoredGlobalNodeId },

    /// Invalid return (outside function).
    #[error(code = "EA304", message = "invalid return")]
    InvalidReturn { node: AnchoredGlobalNodeId },

    /// Missing return on code paths in functions that must return a value.
    #[error(code = "EA305", message = "missing return")]
    MissingReturn { node: AnchoredGlobalNodeId },

    /// Use of uninitialized variable in a read position.
    #[error(code = "EA306", message = "uninitialized variable")]
    UninitializedVariable { node: AnchoredGlobalNodeId },

    /// Incomplete try expression.
    #[error(code = "EA307", message = "try requires a catch or finally")]
    IncompleteTry { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 4xx: Pattern matching
    // -------------------------------------------------------------------------
    /// Non-exhaustive match/switch when exhaustiveness is required.
    #[error(code = "EA400", message = "non-exhaustive match")]
    NonExhaustiveMatch { node: AnchoredGlobalNodeId },

    /// Incomplete pattern.
    #[error(code = "EA401", message = "incomplete pattern")]
    IncompletePattern { node: AnchoredGlobalNodeId },

    /// Conflicting pattern arms.
    #[error(code = "EA402", message = "conflicting pattern")]
    ConflictingPattern {
        node: AnchoredGlobalNodeId,
        other_node: Option<AnchoredGlobalNodeId>,
    },

    // -------------------------------------------------------------------------
    // 5xx: Class / interface / function structure
    // -------------------------------------------------------------------------
    /// Invalid lineage.
    #[error(code = "EA500", message = "invalid lineage")]
    InvalidLineage {
        node: AnchoredGlobalNodeId,
        extends_symbols: Vec<GlobalSymbolId>,
        implements_symbols: Vec<GlobalSymbolId>,
        embedded_symbols: Vec<GlobalSymbolId>,
    },

    /// Invalid constructor.
    #[error(code = "EA501", message = "invalid constructor")]
    InvalidConstructor { node: AnchoredGlobalNodeId },

    /// Invalid interface (e.g., abstract interface).
    #[error(code = "EA502", message = "invalid interface")]
    InvalidInterface { node: AnchoredGlobalNodeId },

    /// Invalid function (e.g., declare function with body).
    #[error(code = "EA503", message = "invalid function")]
    InvalidFunction { node: AnchoredGlobalNodeId },

    /// Invalid method (e.g., abstract method in non-abstract class, abstract method with body).
    #[error(code = "EA504", message = "invalid {abstraction} method")]
    InvalidMethod {
        node: AnchoredGlobalNodeId,
        abstraction: FunctionAbstraction,
    },

    /// Invalid member modifier (e.g., private field with visibility modifier).
    #[error(code = "EA505", message = "invalid member modifier")]
    InvalidMemberModifier { node: AnchoredGlobalNodeId },

    /// Parameter property (visibility/readonly modifier) is only allowed in constructor.
    #[error(
        code = "EA506",
        message = "parameter property is only allowed in a constructor"
    )]
    InvalidParameterProperty { node: AnchoredGlobalNodeId },

    /// Static class blocks cannot have modifiers (other than `static`).
    #[error(
        code = "EA507",
        message = "static class blocks cannot have any modifier"
    )]
    InvalidStaticBlockModifier { node: AnchoredGlobalNodeId },

    /// Inconsistent function override.
    #[error(
        code = "EA508",
        message = "{abstraction} function has inconsistent override"
    )]
    InconsistentFunctionOverride {
        node: AnchoredGlobalNodeId,
        abstraction: FunctionAbstraction,
    },

    // -------------------------------------------------------------------------
    // 6xx: Accessibility
    // -------------------------------------------------------------------------
    /// Inaccessible symbol (private/internal/module boundaries).
    #[error(code = "EA600", message = "'{symbol}' is {visibility}")]
    InaccessibleSymbol {
        node: AnchoredGlobalNodeId,
        visibility: Visibility,
        symbol: GlobalSymbolId,
    },

    // -------------------------------------------------------------------------
    // 7xx: Decorators
    // -------------------------------------------------------------------------
    /// Invalid well-known decorator usage.
    #[error(code = "EA700", message = "invalid well-known decorator: {message}")]
    InvalidWellKnownDecorator {
        node: AnchoredGlobalNodeId,
        message: StringId,
    },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[error(code = "EA900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },
}
