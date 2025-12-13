use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_dir::{
    FunctionAbstraction, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, StaticKey, Visibility,
};
use destack_source::StringId;
use destack_workspace::Program;

/// Errors during the analyze phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Analyze)]
pub enum AnalyzeError {
    /// Wait for task dependency.
    #[error(code = "EA000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EA001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Unsupported node.
    #[error(code = "EA002", message = "unsupported construct")]
    UnsupportedConstruct { node: GlobalNodeIdAny },

    /// Missing type for an expression.
    #[error(code = "EA003", message = "missing type")]
    MissingType { node: GlobalNodeIdAny },

    /// Type is not assignable to the expected type.
    #[error(code = "EA004", message = "type is not assignable")]
    UnassignableType {
        node: GlobalNodeIdAny,
        expected_ty: GlobalTypeId,
        actual_ty: GlobalTypeId,
    },

    /// Inaccessible symbol (private/internal/module boundaries).
    #[error(code = "EA005", message = "inaccessible symbol")]
    InaccessibleSymbol {
        node: GlobalNodeIdAny,
        visibility: Visibility,
        symbol: GlobalSymbolId,
    },

    /// Inconsistent function override.
    #[error(code = "EA006", message = "inconsistent function override")]
    InconsistentFunctionOverride {
        node: GlobalNodeIdAny,
        abstraction: FunctionAbstraction,
    },

    /// Calling non-callable.
    #[error(code = "EA007", message = "calling non-callable")]
    NonCallable { node: GlobalNodeIdAny },

    /// Indexing non-indexable.
    #[error(code = "EA008", message = "indexing non-indexable")]
    NonIndexable { node: GlobalNodeIdAny },

    /// Non-exhaustive match/switch when exhaustiveness is required.
    #[error(code = "EA009", message = "non-exhaustive match")]
    NonExhaustiveMatch { node: GlobalNodeIdAny },

    /// Incomplete pattern.
    #[error(code = "EA010", message = "incomplete pattern")]
    IncompletePattern { node: GlobalNodeIdAny },

    /// Conflicting pattern arms.
    #[error(code = "EA011", message = "conflicting pattern")]
    ConflictingPattern {
        node: GlobalNodeIdAny,
        other_node: Option<GlobalNodeIdAny>,
    },

    /// Missing return on code paths in functions that must return a value.
    #[error(code = "EA012", message = "missing return")]
    MissingReturn { node: GlobalNodeIdAny },

    /// Use of uninitialized variable in a read position.
    #[error(code = "EA013", message = "uninitialized variable")]
    UninitializedVariable { node: GlobalNodeIdAny },

    /// Invalid casts (unsafe or impossible with static rules).
    #[error(code = "EA014", message = "invalid cast")]
    InvalidCast {
        node: GlobalNodeIdAny,
        from_ty: GlobalTypeId,
        to_ty: GlobalTypeId,
    },

    /// No overload found for operator/method with given types.
    #[error(code = "EA015", message = "no matching overload")]
    NoOverload {
        node: GlobalNodeIdAny,
        receiver_ty: GlobalTypeId,
    },

    /// Ambiguous overload: multiple candidates match equally well.
    #[error(code = "EA016", message = "ambiguous overload")]
    AmbiguousOverload {
        node: GlobalNodeIdAny,
        candidates: Vec<GlobalSymbolId>,
    },

    /// Operator not supported for type.
    #[error(code = "EA017", message = "unsupported operator")]
    UnsupportedOperator {
        node: GlobalNodeIdAny,
        ty: GlobalTypeId,
    },

    /// Missing member on type.
    #[error(code = "EA018", message = "missing member")]
    MissingMember {
        node: GlobalNodeIdAny,
        receiver_ty: GlobalTypeId,
        member_key: StaticKey,
    },

    /// Type does not satisfy the expected type (satisfies expression).
    #[error(code = "EA019", message = "unsatisfied type")]
    UnsatisfiedType {
        node: GlobalNodeIdAny,
        expected_ty: GlobalTypeId,
        actual_ty: GlobalTypeId,
    },

    /// Invalid lineage.
    #[error(code = "EA020", message = "invalid lineage")]
    InvalidLineage {
        node: GlobalNodeIdAny,
        extends_symbols: Vec<GlobalSymbolId>,
        implements_symbols: Vec<GlobalSymbolId>,
        embedded_symbols: Vec<GlobalSymbolId>,
    },

    /// Invalid break.
    #[error(code = "EA021", message = "invalid break")]
    InvalidBreak {
        node: GlobalNodeIdAny,
        label: Option<StringId>,
    },

    /// Invalid continue.
    #[error(code = "EA022", message = "invalid continue")]
    InvalidContinue {
        node: GlobalNodeIdAny,
        label: Option<StringId>,
    },

    /// Invalid await.
    #[error(code = "EA023", message = "invalid await")]
    InvalidAwait { node: GlobalNodeIdAny },

    /// Invalid yield.
    #[error(code = "EA024", message = "invalid yield")]
    InvalidYield { node: GlobalNodeIdAny },

    /// Invalid return (outside function).
    #[error(code = "EA025", message = "invalid return")]
    InvalidReturn { node: GlobalNodeIdAny },

    /// Invalid constructor.
    #[error(code = "EA026", message = "invalid constructor")]
    InvalidConstructor { node: GlobalNodeIdAny },

    /// Invalid interface (e.g., abstract interface).
    #[error(code = "EA027", message = "invalid interface")]
    InvalidInterface { node: GlobalNodeIdAny },

    /// Invalid function (e.g., declare function with body).
    #[error(code = "EA028", message = "invalid function")]
    InvalidFunction { node: GlobalNodeIdAny },

    /// Invalid method (e.g., abstract method in non-abstract class, abstract method with body).
    #[error(code = "EA029", message = "invalid method")]
    InvalidMethod {
        node: GlobalNodeIdAny,
        abstraction: FunctionAbstraction,
    },

    /// Invalid member modifier (e.g., private field with visibility modifier).
    #[error(code = "EA030", message = "invalid member modifier")]
    InvalidMemberModifier { node: GlobalNodeIdAny },

    /// Parameter property (visibility/readonly modifier) is only allowed in constructor.
    #[error(
        code = "EA031",
        message = "parameter property is only allowed in a constructor"
    )]
    InvalidParameterProperty { node: GlobalNodeIdAny },

    /// Static class blocks cannot have modifiers (other than `static`).
    #[error(
        code = "EA032",
        message = "static class blocks cannot have any modifier"
    )]
    InvalidStaticBlockModifier { node: GlobalNodeIdAny },
}
