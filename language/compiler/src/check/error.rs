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
    ///
    /// ```ds
    /// const value = _;
    /// ```
    #[diagnostic(code = "EC100", message = "cannot solve constraints")]
    CannotSolve {
        /// Report the node that requires the solution.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Declaration requires an explicit or contextual type annotation.
    ///
    /// ```ds
    /// declare function foo();
    /// ```
    #[diagnostic(code = "EC101", message = "missing type annotation")]
    MissingTypeAnnotation {
        /// Report the declaration that needs a type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Transparent type expansion reached the same type again.
    ///
    /// ```ds
    /// type Loop = Loop;
    /// ```
    #[diagnostic(code = "EC103", message = "type is circular")]
    CircularType {
        /// Report the recursive type reference.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Parsed type form is not part of the language model.
    ///
    /// ```ds
    /// let value: unsupported;
    /// ```
    #[diagnostic(code = "EC104", message = "unsupported type: {name}")]
    UnsupportedType {
        /// Report the unsupported type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The unsupported type spelling.
        name: String,
    },

    // -------------------------------------------------------------------------
    // 2xx: relations
    // -------------------------------------------------------------------------
    /// Source type is not assignable to target type.
    ///
    /// ```ds
    /// let value: string = 1;
    /// ```
    #[diagnostic(code = "EC200", message = "type is not assignable")]
    NotAssignable {
        /// Report the assignment source.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type does not satisfy a required structural or generic constraint.
    ///
    /// ```ds
    /// value satisfies { name: string };
    /// ```
    #[diagnostic(code = "EC201", message = "type does not satisfy constraint")]
    ConstraintNotSatisfied {
        /// Report the constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type does not extend a required base type.
    ///
    /// ```ds
    /// class User extends number {}
    /// ```
    #[diagnostic(code = "EC202", message = "type does not extend required type")]
    DoesNotExtend {
        /// Report the extends clause or constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type does not implement a required contract.
    ///
    /// ```ds
    /// class User implements Serializable {}
    /// ```
    #[diagnostic(code = "EC203", message = "type does not implement required contract")]
    DoesNotImplement {
        /// Report the implements clause or constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Assignment writes through a target that is not writable.
    ///
    /// ```ds
    /// const value = 1;
    /// value = 2;
    /// ```
    #[diagnostic(code = "EC204", message = "assignment target is not writable")]
    NotWritable {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Fresh object literal contains a property that the target cannot accept.
    ///
    /// ```ds
    /// const value: { name: string } = { name: "Ada", extra: true };
    /// ```
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
    ///
    /// ```ds
    /// const value = user as int32;
    /// ```
    #[diagnostic(code = "EC206", message = "type cannot be cast")]
    InvalidCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Intrinsic marker type appears outside a compiler-recognized language item.
    ///
    /// ```ds
    /// let value: intrinsic;
    /// ```
    #[diagnostic(code = "EC207", message = "intrinsic type is not valid here")]
    InvalidIntrinsicType {
        /// Report the intrinsic type expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Const assertion marker appears outside an `as const` expression.
    ///
    /// ```ds
    /// let value: const;
    /// ```
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
    ///
    /// ```ds
    /// user.missing;
    /// ```
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
    ///
    /// ```ds
    /// const value = 1;
    /// value();
    /// ```
    #[diagnostic(code = "EC301", message = "value is not callable")]
    NotCallable {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// No overload matches the supplied arguments.
    ///
    /// ```ds
    /// parse(1, 2, 3);
    /// ```
    #[diagnostic(code = "EC302", message = "no matching call overload")]
    NoMatchingCall {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Member selection has multiple valid targets.
    ///
    /// ```ds
    /// value.name;
    /// ```
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
    ///
    /// ```ds
    /// parse(value);
    /// ```
    #[diagnostic(code = "EC304", message = "ambiguous call")]
    AmbiguousCall {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Selected member is not accessible from the current scope.
    ///
    /// ```ds
    /// user.privateName;
    /// ```
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
    ///
    /// ```ds
    /// user + settings;
    /// ```
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
    ///
    /// ```ds
    /// user === 1;
    /// ```
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

    /// Reference does not resolve to a visible symbol.
    ///
    /// ```ds
    /// missing;
    /// ```
    #[diagnostic(code = "EC308", message = "unresolved reference '{name}'")]
    UnresolvedReference {
        /// Report the reference expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The unresolved reference text.
        name: String,
    },

    /// Reference resolves to more than one visible symbol.
    ///
    /// ```ds
    /// value;
    /// ```
    #[diagnostic(code = "EC309", message = "ambiguous reference '{name}'")]
    AmbiguousReference {
        /// Report the reference expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The ambiguous reference text.
        name: String,
    },

    // -------------------------------------------------------------------------
    // 4xx: expressions
    // -------------------------------------------------------------------------
    /// Runtime condition does not have boolean type.
    ///
    /// ```ds
    /// if (1) {}
    /// ```
    #[diagnostic(code = "EC400", message = "condition requires boolean type")]
    NonBooleanCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Static condition could not be evaluated to a boolean value.
    ///
    /// ```ds
    /// @if("test")
    /// const value = 1;
    /// ```
    #[diagnostic(code = "EC401", message = "static condition requires boolean value")]
    InvalidCondition {
        /// Report the static condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Control flow construct is not valid in its current scope.
    ///
    /// ```ds
    /// break;
    /// ```
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
    ///
    /// ```ds
    /// match (value) {
    ///     true => 1,
    /// }
    /// ```
    #[diagnostic(code = "EC403", message = "pattern match is not exhaustive")]
    NonExhaustivePattern {
        /// Report the match expression.
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
    #[diagnostic(code = "EC404", message = "pattern is unreachable")]
    UnreachablePattern {
        /// Report the unreachable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Local value is used before it is definitely assigned.
    ///
    /// ```ds
    /// let value: int32;
    /// value + 1;
    /// ```
    #[diagnostic(code = "EC405", message = "value is used before assignment")]
    UseBeforeAssigned {
        /// Report the use.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Refutable pattern appears outside a matching context.
    ///
    /// ```ds
    /// let value! = maybe;
    /// ```
    #[diagnostic(
        code = "EC406",
        message = "refutable pattern requires a matching context"
    )]
    RefutablePattern {
        /// Report the refutable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Await expression has an invalid shape for its context.
    ///
    /// ```ds
    /// const value = await promise;
    /// ```
    #[diagnostic(code = "EC407", message = "invalid await expression: {message}")]
    InvalidAwait {
        /// Report the await expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// Describe why the await expression is invalid.
        message: String,
    },

    /// Yield expression has an invalid shape for its context.
    ///
    /// ```ds
    /// yield value;
    /// ```
    #[diagnostic(code = "EC408", message = "invalid yield expression: {message}")]
    InvalidYield {
        /// Report the yield expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// Describe why the yield expression is invalid.
        message: String,
    },

    /// Static operation could not be evaluated.
    ///
    /// ```ds
    /// type Block = [uint8; 1 / 0];
    /// ```
    #[diagnostic(code = "EC409", message = "static evaluation failed: {message}")]
    InvalidStaticOperation {
        /// Report the static operation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// Describe why the evaluation failed.
        message: String,
    },

    // -------------------------------------------------------------------------
    // 5xx: representation
    // -------------------------------------------------------------------------
    /// Type is not concrete and therefore has no layout.
    ///
    /// ```ds
    /// sizeOf<T>();
    /// ```
    #[diagnostic(code = "EC500", message = "type has no concrete layout")]
    LayoutNotConcrete {
        /// Report the layout request.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Representation decorator is not valid for the declaration.
    ///
    /// ```ds
    /// @repr("packed")
    /// interface Shape {}
    /// ```
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
    ///
    /// ```ds
    /// class User {
    ///     override name() {}
    /// }
    /// ```
    #[diagnostic(code = "EC600", message = "invalid override")]
    InvalidOverride {
        /// Report the override declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Concrete type does not implement an abstract member.
    ///
    /// ```ds
    /// class User extends Entity {}
    /// ```
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
    ///
    /// ```ds
    /// new AbstractUser();
    /// ```
    #[diagnostic(code = "EC602", message = "abstract type cannot be constructed")]
    CannotConstructAbstractType {
        /// Report the constructor call.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Method uses an implicit receiver while implicit receivers are disabled.
    ///
    /// ```ds
    /// class User {
    ///     name() {}
    /// }
    /// ```
    #[diagnostic(code = "EC603", message = "method receiver is implicit")]
    ImplicitReceiver {
        /// Report the method declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },
}
