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
    #[diagnostic(code = "EC100", message = "cannot infer a type here")]
    CannotInferType {
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
    /// let value: object;
    /// ```
    #[diagnostic(code = "EC104", message = "unsupported type: {name}")]
    UnsupportedType {
        /// Report the unsupported type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The unsupported type written form.
        name: String,
    },

    /// Generic application supplies more arguments than the declaration takes.
    ///
    /// ```ds
    /// Box<int32, string>;
    /// ```
    #[diagnostic(
        code = "EC105",
        message = "'{name}' takes {expected} generic argument(s), but {supplied} were supplied"
    )]
    WrongGenericArity {
        /// Report the generic application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The applied declaration name.
        name: String,
        /// The declared parameter count.
        expected: usize,
        /// The supplied argument count.
        supplied: usize,
    },

    // -------------------------------------------------------------------------
    // 2xx: relations
    // -------------------------------------------------------------------------
    /// Source type is not assignable to target type.
    ///
    /// ```ds
    /// let value: string = 1;
    /// ```
    #[diagnostic(
        code = "EC200",
        message = "type '{source}' is not assignable to type '{target}'"
    )]
    NotAssignable {
        /// Report the assignment source.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The assigned source type.
        source: String,
        /// The receiving target type.
        target: String,
    },

    /// Type does not satisfy a required structural or generic constraint.
    ///
    /// ```ds
    /// declare const value: {};
    ///
    /// value satisfies { name: string };
    /// ```
    #[diagnostic(
        code = "EC201",
        message = "type '{source}' does not satisfy '{target}'"
    )]
    ConstraintNotSatisfied {
        /// Report the constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The constrained source type.
        source: String,
        /// The required constraint.
        target: String,
    },

    /// Type does not extend a required base type.
    ///
    /// ```ds
    /// class User extends number {}
    /// ```
    #[diagnostic(code = "EC202", message = "type '{source}' does not extend '{target}'")]
    DoesNotExtend {
        /// Report the extends clause or constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The extending source type.
        source: String,
        /// The required base type.
        target: String,
    },

    /// Type does not implement a required interface.
    ///
    /// ```ds
    /// interface Serializable {
    ///     serialize(): string;
    /// }
    ///
    /// class User implements Serializable {}
    /// ```
    #[diagnostic(
        code = "EC203",
        message = "type '{source}' does not implement interface '{target}'"
    )]
    InterfaceNotImplemented {
        /// Report the implements clause or constrained type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The implementing source type.
        source: String,
        /// The required interface.
        target: String,
    },

    /// Assignment target does not designate storage.
    ///
    /// ```ds
    /// let value = 1;
    ///
    /// (value + 1) = 2;
    /// ```
    #[diagnostic(
        code = "EC204",
        message = "assignment target is not a storage location"
    )]
    InvalidAssignmentTarget {
        /// Report the assignment target.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Fresh object literal contains a property that the target cannot accept.
    ///
    /// ```ds
    /// const value: { name: string } = { name: "Ada", extra: true };
    /// ```
    #[diagnostic(
        code = "EC205",
        message = "unknown property '{key}' in object literal for type '{target}'"
    )]
    ExcessProperty {
        /// Report the extra property.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The extra property key.
        key: String,
        /// The receiving target type.
        target: String,
    },

    /// Type cannot be explicitly cast to the requested target type.
    ///
    /// ```ds
    /// const value = "text" as int32;
    /// ```
    #[diagnostic(
        code = "EC206",
        message = "type '{source}' cannot be cast to '{target}'"
    )]
    InvalidCast {
        /// Report the cast expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The cast source type.
        source: String,
        /// The cast target type.
        target: String,
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

    /// Argument type is not assignable to its parameter type.
    ///
    /// ```ds
    /// declare function parse(input: string): int32;
    ///
    /// parse(1);
    /// ```
    #[diagnostic(
        code = "EC209",
        message = "argument of type '{source}' is not assignable to parameter of type '{target}'"
    )]
    ArgumentNotAssignable {
        /// Report the argument expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The argument type.
        source: String,
        /// The parameter type.
        target: String,
    },

    /// Returned type is not assignable to the declared result type.
    ///
    /// ```ds
    /// function f(): string { 1 }
    /// ```
    #[diagnostic(
        code = "EC210",
        message = "type '{source}' is not assignable to the declared result type '{target}'"
    )]
    ReturnNotAssignable {
        /// Report the returned expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The returned type.
        source: String,
        /// The declared result type.
        target: String,
    },

    /// Spread source has no fields to merge.
    ///
    /// ```ds
    /// const merged = { ...1 };
    /// ```
    #[diagnostic(
        code = "EC211",
        message = "type '{source}' cannot be spread into an object literal"
    )]
    SpreadNotObject {
        /// Report the spread expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The spread source type.
        source: String,
    },

    /// Assignment writes to an immutable binding.
    ///
    /// ```ds
    /// const value = 1;
    /// value = 2;
    /// ```
    #[diagnostic(
        code = "EC212",
        message = "cannot assign to immutable binding '{name}'"
    )]
    CannotAssignImmutableBinding {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written binding name.
        name: String,
    },

    /// Assignment writes to an imported binding.
    ///
    /// ```ds
    /// import { value } from "./value.ds";
    /// value = 2;
    /// ```
    #[diagnostic(code = "EC213", message = "cannot assign to imported binding '{name}'")]
    CannotAssignImportedBinding {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written binding name.
        name: String,
    },

    /// Assignment writes to a readonly member.
    ///
    /// ```ds
    /// declare const value: { readonly count: int32 };
    ///
    /// value.count = 2;
    /// ```
    #[diagnostic(
        code = "EC214",
        message = "cannot assign to readonly member '{member}'"
    )]
    CannotAssignReadonlyMember {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written member key.
        member: String,
    },

    /// Assigned object is missing a required property.
    ///
    /// ```ds
    /// const value: { name: string } = {};
    /// ```
    #[diagnostic(
        code = "EC215",
        message = "missing required property '{key}' for type '{target}'"
    )]
    MissingRequiredProperty {
        /// Report the object expression or source type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The missing property key.
        key: String,
        /// The receiving target type.
        target: String,
    },

    /// Writable index signature requires `IndexSet` support.
    ///
    /// ```ds
    /// type Bag = { [key: string]: int32 };
    /// declare function write(bag: Bag): void;
    ///
    /// write({ x: 1 });
    /// ```
    #[diagnostic(
        code = "EC216",
        message = "type '{source}' is missing IndexSet<{key}, {value}> for writable index signature"
    )]
    WritableIndexRequiresIndexSet {
        /// Report the source type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The assigned source type.
        source: String,
        /// The required index key type.
        key: String,
        /// The required index value type.
        value: String,
    },

    // -------------------------------------------------------------------------
    // 3xx: selection
    // -------------------------------------------------------------------------
    /// Receiver type does not contain a selected member.
    ///
    /// ```ds
    /// declare const user: { name: string };
    ///
    /// user.missing;
    /// ```
    #[diagnostic(
        code = "EC300",
        message = "member '{key}' does not exist on type '{receiver}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    MissingMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
        /// The receiver type.
        receiver: String,
        /// The closest visible member key.
        suggestion: Option<String>,
    },

    /// Value is not callable.
    ///
    /// ```ds
    /// const value = 1;
    /// value();
    /// ```
    #[diagnostic(code = "EC301", message = "value of type '{ty}' is not callable")]
    NotCallable {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The callee type.
        ty: String,
    },

    /// No overload matches the supplied arguments.
    ///
    /// ```ds
    /// declare function parse(input: string): int32;
    ///
    /// parse(1, 2, 3);
    /// ```
    #[diagnostic(
        code = "EC302",
        message = "no overload matches arguments ({arguments})"
    )]
    NoMatchingCall {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The supplied argument types.
        arguments: String,
    },

    /// Member selection has multiple valid targets.
    ///
    /// ```ds
    /// value.name;
    /// ```
    #[diagnostic(code = "EC303", message = "member '{key}' is ambiguous")]
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
    /// class User {
    ///     private value: int32 = 0;
    /// }
    ///
    /// const user = new User();
    /// user.value;
    /// ```
    #[diagnostic(code = "EC305", message = "member '{key}' is {visibility}")]
    InaccessibleMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
        /// The declared visibility.
        visibility: String,
    },

    /// No operator overload matches the supplied operands.
    ///
    /// ```ds
    /// 1 + true;
    /// ```
    #[diagnostic(
        code = "EC306",
        message = "operator '{operator}' is not defined for {operands}"
    )]
    NoMatchingOperator {
        /// Report the operator expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected operator.
        operator: String,
        /// The supplied operand types.
        operands: String,
    },

    /// Strict equality operands do not have identity-compatible types.
    ///
    /// ```ds
    /// class User {}
    ///
    /// declare const user: User;
    ///
    /// user === 1;
    /// ```
    #[diagnostic(
        code = "EC307",
        message = "this comparison is unintentional: types '{left}' and '{right}' have no overlap"
    )]
    InvalidStrictEquality {
        /// Report the strict equality expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The left operand type.
        left: String,
        /// The right operand type.
        right: String,
    },

    /// Reference does not resolve to a visible symbol.
    ///
    /// ```ds
    /// missing;
    /// ```
    #[diagnostic(
        code = "EC308",
        message = "cannot find '{name}'",
        optional_message = "; did you mean '{suggestion}'?"
    )]
    UnresolvedReference {
        /// Report the reference expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The unresolved reference text.
        name: String,
        /// The closest visible reference name.
        suggestion: Option<String>,
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

    /// Decorator target is not a static declaration name.
    ///
    /// ```ds
    /// @value.field
    /// const decorated = 1;
    /// ```
    #[diagnostic(code = "EC310", message = "decorator must name a declaration")]
    InvalidDecoratorTarget {
        /// Report the decorator target expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Member access reads through a possibly nullish value.
    ///
    /// ```ds
    /// class User {
    ///     name: string = "";
    /// }
    ///
    /// declare const user: User | undefined;
    /// user.name;
    /// ```
    #[diagnostic(code = "EC312", message = "value is possibly {nullish}")]
    PossiblyNullish {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The nullish part of the receiver.
        nullish: String,
    },

    /// Type cannot be constructed with `new`.
    ///
    /// ```ds
    /// struct Point {}
    ///
    /// new Point();
    /// ```
    #[diagnostic(
        code = "EC313",
        message = "type '{ty}' cannot be constructed with 'new'{hint}"
    )]
    NotConstructible {
        /// Report the construct expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The constructed type.
        ty: String,
        /// Construction guidance for the type's kind.
        hint: String,
    },

    /// Call supplies the wrong number of arguments.
    ///
    /// ```ds
    /// function pair(a: int32, b: int32) {}
    /// pair(1);
    /// ```
    #[diagnostic(
        code = "EC314",
        message = "expected {expected}, but got {supplied} argument(s)"
    )]
    WrongArgumentCount {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The accepted argument count phrase, possibly a range.
        expected: String,
        /// The supplied argument count.
        supplied: usize,
    },

    /// Value does not support indexed access.
    ///
    /// ```ds
    /// type Value = int32["name"];
    /// ```
    #[diagnostic(code = "EC315", message = "type '{receiver}' cannot be indexed")]
    InvalidIndexReceiver {
        /// Report the index expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The indexed receiver type.
        receiver: String,
    },

    /// Index key type is not valid for the receiver.
    ///
    /// ```ds
    /// type User = { name: string };
    ///
    /// type Value = User["missing"];
    /// ```
    #[diagnostic(
        code = "EC316",
        message = "type '{receiver}' cannot be indexed by type '{key}'"
    )]
    InvalidIndexKey {
        /// Report the index expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The indexed receiver type.
        receiver: String,
        /// The supplied index key type.
        key: String,
    },

    // -------------------------------------------------------------------------
    // 4xx: expressions
    // -------------------------------------------------------------------------
    /// Runtime condition does not have boolean type.
    ///
    /// ```ds
    /// if (1) {}
    /// ```
    #[diagnostic(
        code = "EC400",
        message = "condition must be boolean, found '{actual}'"
    )]
    NonBooleanCondition {
        /// Report the condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The condition type.
        actual: String,
    },

    /// Static condition could not be evaluated to a boolean value.
    ///
    /// ```ds
    /// @if("test")
    /// const value = 1;
    /// ```
    #[diagnostic(
        code = "EC401",
        message = "static condition must evaluate to a boolean"
    )]
    InvalidStaticCondition {
        /// Report the static condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Break expression has no target.
    ///
    /// ```ds
    /// break;
    /// ```
    #[diagnostic(code = "EC402", message = "break statement has no target")]
    BreakOutsideControlTarget {
        /// Report the break expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern matching does not cover every possible value.
    ///
    /// ```ds
    /// declare const value: true | false;
    ///
    /// match (value) {
    ///     true => 1,
    /// }
    /// ```
    #[diagnostic(
        code = "EC403",
        message = "match is not exhaustive: '{missing}' is not covered"
    )]
    NonExhaustivePattern {
        /// Report the match expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// One uncovered value or type.
        missing: String,
    },

    /// Local value is used before it is definitely assigned.
    ///
    /// ```ds
    /// let value: int32;
    /// value + 1;
    /// ```
    #[diagnostic(code = "EC405", message = "'{name}' is used before being assigned")]
    UseBeforeAssigned {
        /// Report the use.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The used binding name.
        name: String,
    },

    /// Refutable pattern appears outside a matching context.
    ///
    /// ```ds
    /// declare const state: "ready" | "error";
    ///
    /// let "ready" = state;
    /// ```
    #[diagnostic(
        code = "EC406",
        message = "refutable pattern in binding position: '{missing}' is not covered"
    )]
    RefutablePattern {
        /// Report the refutable pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// One uncovered value or type.
        missing: String,
    },

    /// Await expression appears outside an async context.
    ///
    /// ```ds
    /// declare const promise: Promise<int32>;
    ///
    /// const value = await promise;
    /// ```
    #[diagnostic(code = "EC407", message = "await expression requires an async context")]
    AwaitOutsideAsyncContext {
        /// Report the await expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Yield expression appears outside a generator.
    ///
    /// ```ds
    /// declare const value: int32;
    ///
    /// yield value;
    /// ```
    #[diagnostic(code = "EC408", message = "yield expression requires a generator")]
    YieldOutsideGenerator {
        /// Report the yield expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
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

    /// Try operator applies to a value that is neither Try nor nullish.
    ///
    /// ```ds
    /// const value = 1?;
    /// ```
    #[diagnostic(
        code = "EC410",
        message = "'{operator}' requires a Try carrier or nullish value, found '{ty}'"
    )]
    InvalidTryOperand {
        /// Report the try expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The applied operator.
        operator: String,
        /// The tried value type.
        ty: String,
    },

    /// Nominal pattern names a tag that is not a nominal type.
    ///
    /// ```ds
    /// type Point = { x: int32; y: int32 };
    ///
    /// match (value) {
    ///     Point { x, y } => x + y,
    /// }
    /// ```
    #[diagnostic(code = "EC411", message = "pattern tag '{ty}' is not a nominal type")]
    InvalidPatternTag {
        /// Report the pattern tag.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The non-nominal tag type.
        ty: String,
    },

    /// Continue expression has no target loop.
    ///
    /// ```ds
    /// continue;
    /// ```
    #[diagnostic(code = "EC412", message = "continue statement has no target")]
    ContinueOutsideLoop {
        /// Report the continue expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Return expression appears outside a function body.
    ///
    /// ```ds
    /// return;
    /// ```
    #[diagnostic(code = "EC413", message = "return statement is outside a function")]
    ReturnOutsideFunction {
        /// Report the return expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// This expression appears where no receiver is available.
    ///
    /// ```ds
    /// const value = this;
    /// ```
    #[diagnostic(code = "EC414", message = "'this' is not available here")]
    ThisOutsideReceiver {
        /// Report the this expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Super expression appears where no superclass receiver is available.
    ///
    /// ```ds
    /// const value = super;
    /// ```
    #[diagnostic(code = "EC415", message = "'super' is not available here")]
    SuperOutsideClass {
        /// Report the super expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Let-else fallback can complete normally.
    ///
    /// ```ds
    /// declare const status: "ready" | "error";
    ///
    /// let "ready" = status else { 0 };
    /// ```
    #[diagnostic(code = "EC416", message = "else branch of let-else must diverge")]
    LetElseBranchCanComplete {
        /// Report the else branch.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Tree expression is not valid in checked expressions.
    ///
    /// ```ds
    /// tree { value }
    /// ```
    #[diagnostic(code = "EC417", message = "tree expression is not supported here")]
    UnsupportedTreeExpression {
        /// Report the tree expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Expression pattern did not close to a literal.
    ///
    /// ```ds
    /// declare const settings: { ready: string };
    ///
    /// match (value) {
    ///     settings.ready => 1,
    /// }
    /// ```
    #[diagnostic(code = "EC418", message = "expression pattern must close to a literal")]
    ExpressionPatternNotLiteral {
        /// Report the expression pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Yield delegation has no delegated value.
    ///
    /// ```ds
    /// yield*;
    /// ```
    #[diagnostic(code = "EC419", message = "yield* expression requires a value")]
    YieldDelegateMissingValue {
        /// Report the yield expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Try propagation appears outside a function body.
    ///
    /// ```ds
    /// 1?;
    /// ```
    #[diagnostic(
        code = "EC420",
        message = "'?' can only propagate from a function body"
    )]
    TryOutsideFunction {
        /// Report the try expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// For-of source is not iterable.
    ///
    /// ```ds
    /// for (const value of 1) {}
    /// ```
    #[diagnostic(code = "EC421", message = "for-of source must be iterable")]
    ForOfSourceNotIterable {
        /// Report the for-of expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// For-in source is not object-shaped.
    ///
    /// ```ds
    /// for (const key in 1) {}
    /// ```
    #[diagnostic(code = "EC422", message = "for-in source must be object-shaped")]
    ForInSourceNotObjectShaped {
        /// Report the for-in expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern tries to destructure a value that has no object shape.
    ///
    /// ```ds
    /// const { value } = 1;
    /// ```
    #[diagnostic(
        code = "EC423",
        message = "type '{source}' cannot be destructured as an object pattern"
    )]
    PatternSourceNotObjectShaped {
        /// Report the object pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The matched source type.
        source: String,
    },

    /// Pattern tries to destructure a value that has no tuple shape.
    ///
    /// ```ds
    /// declare const value: { x: int32; y: int32 };
    ///
    /// const (left, right) = value;
    /// ```
    #[diagnostic(
        code = "EC424",
        message = "type '{source}' cannot be destructured as a tuple pattern"
    )]
    PatternSourceNotTupleShaped {
        /// Report the tuple pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The matched source type.
        source: String,
    },

    /// Pattern tries to destructure a value that has no sequence shape.
    ///
    /// ```ds
    /// declare const value: { x: int32; y: int32 };
    ///
    /// const [head, ...tail] = value;
    /// ```
    #[diagnostic(
        code = "EC425",
        message = "type '{source}' cannot be destructured as a sequence pattern"
    )]
    PatternSourceNotSequenceShaped {
        /// Report the sequence pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The matched source type.
        source: String,
    },

    /// Pattern names a field that does not exist on the matched type.
    ///
    /// ```ds
    /// declare const value: { name: string };
    ///
    /// const { missing } = value;
    /// ```
    #[diagnostic(
        code = "EC426",
        message = "pattern field '{key}' does not exist on type '{receiver}'"
    )]
    PatternFieldMissing {
        /// Report the missing field pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected field key.
        key: String,
        /// The matched receiver type.
        receiver: String,
    },

    /// Nominal object pattern names a member that is not a field.
    ///
    /// ```ds
    /// match (user) {
    ///     User { displayName } => displayName
    /// }
    /// ```
    #[diagnostic(
        code = "EC427",
        message = "member '{key}' on type '{receiver}' is not a field"
    )]
    PatternMemberNotField {
        /// Report the non-field pattern member.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
        /// The matched receiver type.
        receiver: String,
    },

    /// Pattern repeats the same field in one destructuring shape.
    ///
    /// ```ds
    /// declare const user: { name: string };
    ///
    /// const { name, name: alias } = user;
    /// ```
    #[diagnostic(
        code = "EC428",
        message = "field '{key}' appears more than once in pattern"
    )]
    DuplicatePatternField {
        /// Report the repeated field.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The repeated field key.
        key: String,
    },

    /// Pattern binds the same name more than once.
    ///
    /// ```ds
    /// declare const pair: { left: int32; right: int32 };
    ///
    /// const { left: value, right: value } = pair;
    /// ```
    #[diagnostic(
        code = "EC429",
        message = "binding '{name}' appears more than once in pattern"
    )]
    DuplicatePatternBinding {
        /// Report the repeated binding.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The repeated binding name.
        name: String,
    },

    /// Rest pattern appears before another field.
    ///
    /// ```ds
    /// declare const values: int32[];
    ///
    /// const [head, ...middle, tail] = values;
    /// ```
    #[diagnostic(code = "EC430", message = "rest pattern must be last")]
    RestPatternNotLast {
        /// Report the rest pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Pattern contains more than one rest field.
    ///
    /// ```ds
    /// declare const values: int32[];
    ///
    /// const [head, ...tail, ...rest] = values;
    /// ```
    #[diagnostic(code = "EC431", message = "pattern can contain at most one rest field")]
    MultipleRestPatterns {
        /// Report the extra rest pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Computed pattern key is not valid for the matched source.
    ///
    /// ```ds
    /// declare const key: string;
    /// declare const point: { x: int32 };
    ///
    /// const { [key]: value } = point;
    /// ```
    #[diagnostic(
        code = "EC432",
        message = "computed pattern key is not valid for the source type"
    )]
    ComputedPatternKeyNotValid {
        /// Report the computed key expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Range pattern applies to a non-scalar domain.
    ///
    /// ```ds
    /// declare const value: { min: int32; max: int32 };
    ///
    /// match (value) {
    ///     0..10 => true
    /// }
    /// ```
    #[diagnostic(code = "EC433", message = "range pattern cannot match type '{domain}'")]
    InvalidRangePatternDomain {
        /// Report the range pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The matched domain type.
        domain: String,
    },

    /// Range pattern bound does not close to a valid scalar literal.
    ///
    /// ```ds
    /// declare const start: int32;
    /// declare const end: int32;
    ///
    /// match (value) {
    ///     start..end => true
    /// }
    /// ```
    #[diagnostic(
        code = "EC434",
        message = "range pattern bound must close to an integer, bigint, or char literal"
    )]
    InvalidRangePatternBound {
        /// Report the range bound expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Union pattern alternatives bind incompatible names or forms.
    ///
    /// ```ds
    /// match (result) {
    ///     Ok(value) | Err(error) => value
    /// }
    /// ```
    #[diagnostic(
        code = "EC435",
        message = "union pattern alternatives must bind the same names with the same forms"
    )]
    PatternAlternativeBindingMismatch {
        /// Report the union pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Destructuring assignment is used with a compound assignment operator.
    ///
    /// ```ds
    /// [left, right] += values;
    /// ```
    #[diagnostic(
        code = "EC436",
        message = "destructuring assignment only supports plain '='"
    )]
    DestructuringAssignmentRequiresPlainAssignment {
        /// Report the assignment pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Refutable pattern appears as a catch binding.
    ///
    /// ```ds
    /// try {
    ///     read()?
    /// } catch ("missing") {}
    /// ```
    #[diagnostic(
        code = "EC437",
        message = "catch pattern must be irrefutable: '{missing}' is not covered"
    )]
    RefutableCatchPattern {
        /// Report the refutable catch pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// One uncovered value or type.
        missing: String,
    },

    // -------------------------------------------------------------------------
    // 5xx: representation
    // -------------------------------------------------------------------------
    /// Type is not concrete and therefore has no layout.
    ///
    /// ```ds
    /// function size<T>(): usize {
    ///     return comptime sizeOf<T>();
    /// }
    /// ```
    #[diagnostic(code = "EC500", message = "type '{ty}' has no concrete layout")]
    LayoutNotConcrete {
        /// Report the layout request.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The non-concrete type.
        ty: String,
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

    /// Interval type has no finite bounds.
    ///
    /// ```ds
    /// type Values = 1..;
    /// ```
    #[diagnostic(code = "EC502", message = "interval type must be bounded")]
    UnboundedIntervalType {
        /// Report the interval type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Interval type uses a non-discrete scalar domain.
    ///
    /// ```ds
    /// type Values = 0.0..1.0;
    /// ```
    #[diagnostic(
        code = "EC503",
        message = "interval type bounds must be integer, bigint, or char literals"
    )]
    InvalidIntervalDomain {
        /// Report the interval type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Dynamic erasure requires a dynamic-safe constraint.
    ///
    /// ```ds
    /// const value: Dynamic<<T>(T) => T>;
    /// ```
    #[diagnostic(code = "EC504", message = "type '{ty}' is not dynamic-safe")]
    DynamicSafetyNotSatisfied {
        /// Report the erased type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The type that failed dynamic-safety checking.
        ty: String,
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
    #[diagnostic(
        code = "EC600",
        message = "'{member}' does not override an inherited member"
    )]
    InvalidOverride {
        /// Report the override declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The overriding member name.
        member: String,
    },

    /// Concrete type does not implement an abstract member.
    ///
    /// ```ds
    /// abstract class Entity {
    ///     abstract id(): string;
    /// }
    ///
    /// class User extends Entity {}
    /// ```
    #[diagnostic(
        code = "EC601",
        message = "abstract member '{member}' is not implemented"
    )]
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
    /// abstract class AbstractUser {}
    ///
    /// new AbstractUser();
    /// ```
    #[diagnostic(
        code = "EC602",
        message = "abstract class '{ty}' cannot be constructed"
    )]
    CannotConstructAbstractType {
        /// Report the constructor call.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The abstract class name.
        ty: String,
    },

    /// Method uses an implicit receiver while implicit receivers are disabled.
    ///
    /// ```ds
    /// class User {
    ///     name() {}
    /// }
    /// ```
    #[diagnostic(code = "EC603", message = "method must spell its receiver explicitly")]
    MissingExplicitReceiver {
        /// Report the method declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Two implementations claim the same interface for the same type.
    ///
    /// ```ds
    /// newtype interface Show {}
    /// class User {}
    ///
    /// extension of User implements Show {}
    /// extension of User implements Show {}
    /// ```
    #[diagnostic(
        code = "EC604",
        message = "conflicting implementations of interface '{interface}' for type '{ty}'"
    )]
    ConflictingImplementation {
        /// Report the later implementation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The implemented interface.
        interface: String,
        /// The implementing type.
        ty: String,
    },

    /// Blanket implementation over a bare parameter is declared outside
    /// the interface's package.
    ///
    /// ```ds
    /// newtype interface Equal {}
    /// newtype interface PartialEqual {}
    ///
    /// extension<T: Equal> of T implements PartialEqual {}
    /// ```
    #[diagnostic(
        code = "EC605",
        message = "blanket implementation over a bare parameter must live in the package declaring interface '{interface}'"
    )]
    ForeignBlanketImplementation {
        /// Report the blanket implementation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The implemented interface.
        interface: String,
    },

    /// Member shadows an inherited member without the override modifier.
    ///
    /// ```ds
    /// class User {
    ///     show(): string {}
    /// }
    ///
    /// class Admin extends User {
    ///     show(): string {}
    /// }
    /// ```
    #[diagnostic(
        code = "EC606",
        message = "'{member}' shadows an inherited member and must be declared 'override'"
    )]
    MissingOverride {
        /// Report the shadowing member.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The shadowing member name.
        member: String,
    },

    /// Override targets an inherited member that is not overridable.
    ///
    /// ```ds
    /// class User {
    ///     show(): string {}
    /// }
    ///
    /// class Admin extends User {
    ///     override show(): string {}
    /// }
    /// ```
    #[diagnostic(
        code = "EC607",
        message = "cannot override '{member}': the inherited member is not virtual"
    )]
    OverrideNotVirtual {
        /// Report the override declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The overridden member name.
        member: String,
    },

    /// Class extends a final base class.
    ///
    /// ```ds
    /// final class FinalUser {}
    ///
    /// class Admin extends FinalUser {}
    /// ```
    #[diagnostic(code = "EC608", message = "final class '{ty}' cannot be extended")]
    FinalClassExtended {
        /// Report the extending declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The final base class name.
        ty: String,
    },

    /// Abstract member is declared in a concrete class.
    ///
    /// ```ds
    /// class User {
    ///     abstract show(): string;
    /// }
    /// ```
    #[diagnostic(
        code = "EC609",
        message = "abstract member '{member}' requires an abstract class"
    )]
    AbstractMemberInConcreteClass {
        /// Report the abstract member.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The abstract member name.
        member: String,
    },

    /// Override declaration is not assignable to the inherited member.
    ///
    /// ```ds
    /// class User {
    ///     virtual show(): string {}
    /// }
    ///
    /// class Admin extends User {
    ///     override show(): int32 {}
    /// }
    /// ```
    #[diagnostic(
        code = "EC610",
        message = "override '{member}' has type '{source}', which is not assignable to the inherited type '{target}'"
    )]
    IncompatibleOverride {
        /// Report the override declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The overriding member name.
        member: String,
        /// The overriding member type.
        source: String,
        /// The inherited member type.
        target: String,
    },

    /// Concrete callable declaration has no body.
    ///
    /// ```ds
    /// function parse(input: string): int32;
    /// ```
    #[diagnostic(code = "EC611", message = "declaration '{name}' requires a body")]
    MissingDeclarationBody {
        /// Report the bodyless declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The bodyless declaration name.
        name: String,
    },

    /// Class field is not definitely initialized.
    ///
    /// ```ds
    /// class User {
    ///     name: string;
    /// }
    /// ```
    #[diagnostic(
        code = "EC613",
        message = "field '{field}' is not initialized on every constructor path"
    )]
    FieldNotDefinitelyInitialized {
        /// Report the field declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The uninitialized field name.
        field: String,
    },

    /// Ambient signature elides a result lifetime.
    ///
    /// ```ds
    /// declare function only(value: &Node): &Node;
    /// ```
    #[diagnostic(
        code = "EC614",
        message = "ambient signatures must spell result lifetimes explicitly"
    )]
    AmbientLifetimeElided {
        /// Report the ambient declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },
}
