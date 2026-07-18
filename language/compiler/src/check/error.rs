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
    #[diagnostic(
        code = "EC100",
        message = "cannot infer a type here",
        help = "annotate the type explicitly"
    )]
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

    /// Type query operand is not a value reference path.
    ///
    /// ```ds
    /// type T = typeof call();
    /// ```
    #[diagnostic(
        code = "EC106",
        message = "typeof type query requires a value reference"
    )]
    InvalidTypeQuery {
        /// Report the invalid query operand.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
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
    #[diagnostic(code = "EC204", message = "assignment target is not a writable place")]
    InvalidAssignmentTarget {
        /// Report the assignment target.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Direct object literal contains a property that the target cannot accept.
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
        message = "cannot assign to immutable binding '{name}'",
        help = "declare '{name}' with 'let' to allow reassignment"
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
        message = "type '{source}' is missing IndexSet<{key}> with input '{value}' for writable index signature"
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

    /// Borrow expression requests access its source never grants.
    ///
    /// ```ds
    /// declare const user: shared User;
    ///
    /// const view = &exclusive user;
    /// ```
    #[diagnostic(
        code = "EC217",
        message = "'{access}' access is not granted by a value of type '{source}'"
    )]
    BorrowAccessNotGranted {
        /// Report the borrow expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The requested access.
        access: String,
        /// The borrowed source type.
        source: String,
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

    /// No constructor matches the supplied arguments.
    ///
    /// ```ds
    /// new User(true);
    /// ```
    #[diagnostic(
        code = "EC311",
        message = "no constructor matches arguments ({arguments})"
    )]
    NoMatchingConstruct {
        /// Report the construct expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The supplied argument types.
        arguments: String,
    },

    /// A derive argument is not a registered derive provider.
    ///
    /// ```ds
    /// @derive(ordinaryValue)
    /// newtype Shape = { kind: "shape" };
    /// ```
    #[diagnostic(code = "EC326", message = "type '{provider}' is not a derive provider")]
    InvalidDeriveProvider {
        /// Report the derive provider argument.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected provider type.
        provider: String,
    },

    /// A derive provider does not support the annotated declaration.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// struct Shape {}
    /// ```
    #[diagnostic(
        code = "EC327",
        message = "'{provider}' cannot be derived for this declaration"
    )]
    InvalidDeriveTarget {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected derive provider.
        provider: String,
    },

    /// A declaration selects the same derive provider more than once.
    ///
    /// ```ds
    /// @derive(Tagged, Tagged)
    /// newtype Shape = { kind: "shape" };
    /// ```
    #[diagnostic(code = "EC328", message = "duplicate derive provider '{provider}'")]
    DuplicateDeriveProvider {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The duplicated derive provider.
        provider: String,
    },

    /// One Tagged backing arm has no string literal discriminant.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = { kind: string };
    /// ```
    #[diagnostic(
        code = "EC329",
        message = "Tagged backing arm must declare a string literal '{discriminant}' field"
    )]
    InvalidTaggedVariant {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The configured discriminant field.
        discriminant: String,
    },

    /// A Tagged discriminant cannot produce a declaration member name.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = { kind: "---" };
    /// ```
    #[diagnostic(
        code = "EC330",
        message = "Tagged discriminant '{discriminant}' does not produce a valid case name"
    )]
    InvalidTaggedCase {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected discriminant value.
        discriminant: String,
    },

    /// Two Tagged backing arms select the same case name.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = { kind: "shape" } | { kind: "Shape" };
    /// ```
    #[diagnostic(code = "EC331", message = "duplicate Tagged case '{key}'")]
    DuplicateTaggedCase {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The duplicated case name.
        key: String,
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

    /// Decorator target does not name one newtype declaration.
    ///
    /// ```ds
    /// @value.field
    /// const decorated = 1;
    /// ```
    #[diagnostic(code = "EC310", message = "decorator must name a newtype declaration")]
    InvalidDecoratorTarget {
        /// Report the decorator target expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A closure has more than one capture directive.
    ///
    /// ```ds
    /// @capture("copy")
    /// @capture("move")
    /// const closure = () => value;
    /// ```
    #[diagnostic(code = "EC325", message = "duplicate capture decorator")]
    DuplicateCaptureDecorator {
        /// Report the duplicate capture decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A capture decorator does not annotate a declared function value.
    ///
    /// ```ds
    /// @capture("copy")
    /// const value = 1;
    /// ```
    #[diagnostic(
        code = "EC324",
        message = "capture decorator requires a declared function value"
    )]
    InvalidCaptureTarget {
        /// Report the capture decorator.
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
    #[diagnostic(
        code = "EC312",
        message = "value is possibly {nullish}",
        help = "narrow the value with a check or access it with '?.'"
    )]
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

    /// Type cannot be constructed through an inferred call head.
    ///
    /// ```ds
    /// const value: string = _(1);
    /// ```
    #[diagnostic(
        code = "EC323",
        message = "type '{ty}' cannot be constructed with '_(...)'"
    )]
    InvalidInferredConstructTarget {
        /// Report the construct expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The expected target type.
        ty: String,
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

    /// `instanceof` target is not a class declaration.
    ///
    /// ```ds
    /// interface Named {}
    ///
    /// value instanceof Named;
    /// ```
    #[diagnostic(
        code = "EC317",
        message = "right-hand side of 'instanceof' must be a class"
    )]
    InstanceOfTargetNotClass {
        /// Report the `instanceof` expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// `instanceof` can never hold for the supplied value type.
    ///
    /// ```ds
    /// class User {}
    ///
    /// declare const name: string;
    ///
    /// name instanceof User;
    /// ```
    #[diagnostic(
        code = "EC318",
        message = "type '{source}' can never be an instance of '{target}'"
    )]
    ImpossibleInstanceOf {
        /// Report the tested value.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The tested value type.
        source: String,
        /// The target class type.
        target: String,
    },

    /// `is` can never hold for the supplied value type.
    ///
    /// ```ds
    /// declare const value: string;
    ///
    /// value is int32;
    /// ```
    #[diagnostic(
        code = "EC319",
        message = "type '{source}' can never satisfy runtime check '{target}'"
    )]
    ImpossibleIs {
        /// Report the tested value.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The tested value type.
        source: String,
        /// The checked target type.
        target: String,
    },

    /// `is` target cannot be tested at runtime.
    ///
    /// ```ds
    /// declare const value: unknown;
    ///
    /// value is &User;
    /// ```
    #[diagnostic(
        code = "EC320",
        message = "type '{target}' cannot be tested at runtime"
    )]
    RuntimePredicateNotTestable {
        /// Report the checked target.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The checked target type.
        target: String,
    },

    /// Member access reads a property that only has a setter.
    ///
    /// ```ds
    /// interface Sink {
    ///     set value(next: int32);
    /// }
    ///
    /// declare const sink: Sink;
    /// sink.value;
    /// ```
    #[diagnostic(code = "EC321", message = "member '{member}' is write-only")]
    CannotReadWriteOnlyMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        member: String,
    },

    /// Call receiver does not satisfy the method's declared `this` parameter.
    ///
    /// ```ds
    /// extension of Buffer {
    ///     grow(this: &exclusive Buffer): void {}
    ///
    ///     peek(this: &readonly Buffer): void {
    ///         this.grow();
    ///     }
    /// }
    /// ```
    #[diagnostic(
        code = "EC322",
        message = "receiver type '{source}' is not assignable to the method's 'this' type '{target}'"
    )]
    ReceiverNotAssignable {
        /// Report the call expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The supplied receiver type.
        source: String,
        /// The declared `this` parameter type.
        target: String,
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

    /// Static inclusion condition could not be decided statically.
    ///
    /// ```ds
    /// function f<comptime Enabled: boolean>() {
    ///     @if(Enabled)
    ///     const value = 1;
    /// }
    /// ```
    #[diagnostic(
        code = "EC404",
        message = "static @if condition must be statically decidable"
    )]
    UndecidableStaticCondition {
        /// Report the static condition expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Static value expression is not in the static subset.
    ///
    /// ```ds
    /// type Value<comptime N: number = runtimeValue> = N;
    /// ```
    #[diagnostic(code = "EC440", message = "static value must be statically decidable")]
    UndecidableStaticValue {
        /// Report the static value expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Static guard is not invoked in its intrinsic form.
    ///
    /// ```ds
    /// @if<boolean>(true)
    /// const value = 1;
    /// ```
    #[diagnostic(code = "EC444", message = "`@if` must be invoked as `@if(condition)`")]
    InvalidStaticIfInvocation {
        /// Report the malformed `@if` decorator.
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

    /// Break carries a value outside a `loop` or labeled block.
    ///
    /// ```ds
    /// while (true) { break 1; }
    /// ```
    #[diagnostic(
        code = "EC441",
        message = "break with a value can only target a `loop` or labeled block"
    )]
    BreakValueOutsideLoop {
        /// Report the break expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Declared generic parameter never occurs in its declaration.
    ///
    /// ```ds
    /// class Tag<T> {}
    /// ```
    #[diagnostic(code = "EC442", message = "generic parameter '{name}' is never used")]
    UnusedGenericParameter {
        /// Report the parameter declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The unused parameter name.
        name: String,
    },

    /// Declared variance conflicts with the parameter's derived use.
    ///
    /// ```ds
    /// class Evil<out T> {
    ///     slot: T;
    /// }
    /// ```
    #[diagnostic(
        code = "EC443",
        message = "generic parameter '{name}' is used {usage} and cannot be declared '{declared}'"
    )]
    VarianceConflict {
        /// Report the parameter declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The conflicting parameter name.
        name: String,
        /// The derived use wording, like "invariantly".
        usage: String,
        /// The declared modifier text, like "out".
        declared: String,
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

    /// Tree expression has no active builder.
    ///
    /// ```ds
    /// <View />
    /// ```
    #[diagnostic(
        code = "EC417",
        message = "tree expression requires an active tree builder"
    )]
    MissingTreeBuilder {
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

    /// Variant pattern belongs to a different nominal type.
    ///
    /// ```ds
    /// match (status) {
    ///     Other.Done(value) => value
    /// }
    /// ```
    #[diagnostic(
        code = "EC438",
        message = "variant '{variant}' is not a variant of type '{source}'"
    )]
    PatternVariantNotInType {
        /// Report the variant pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written variant head.
        variant: String,
        /// The matched source type.
        source: String,
    },

    /// Variant pattern names a variant that does not exist.
    ///
    /// ```ds
    /// match (status) {
    ///     Status.Done(value) => value
    /// }
    /// ```
    #[diagnostic(
        code = "EC439",
        message = "variant '{variant}' does not exist on type '{owner}'"
    )]
    PatternVariantMissing {
        /// Report the variant pattern.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The missing variant name.
        variant: String,
        /// The owner type.
        owner: String,
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

    /// A declaration does not support the selected representation family.
    #[diagnostic(
        code = "EC501",
        message = "representation '{representation}' is not supported by this declaration"
    )]
    UnsupportedRepresentation {
        /// Report the representation decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected representation.
        representation: String,
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

    /// Non-exclusive writes require overwrite-stable storage.
    ///
    /// ```ds
    /// *borrow = value;
    /// ```
    #[diagnostic(
        code = "EC505",
        message = "type '{ty}' is not safe to overwrite through non-exclusive access"
    )]
    OverwriteStabilityNotSatisfied {
        /// Report the overwritten type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The type that failed overwrite-stability checking.
        ty: String,
    },

    /// Shared storage retains a safe reference into local storage.
    ///
    /// ```ds
    /// shared struct State { user: local User }
    /// ```
    #[diagnostic(
        code = "EC506",
        message = "shared space cannot hold references into local space"
    )]
    LocalReferenceInSharedStorage {
        /// Report the stored local reference.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A C enum has string variant values.
    #[diagnostic(
        code = "EC507",
        message = "C enum representation requires integer variant values"
    )]
    NonIntegerCEnum {
        /// Report the representation decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// An enum variant value does not fit the selected integer representation.
    #[diagnostic(
        code = "EC508",
        message = "enum value {value} does not fit representation '{representation}'"
    )]
    EnumValueOutsideRepresentation {
        /// Report the representation decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected enum value.
        value: String,
        /// The selected integer representation.
        representation: String,
    },

    /// A declaration carries more than one representation decorator.
    #[diagnostic(code = "EC509", message = "duplicate representation decorator")]
    DuplicateRepresentationDecorator {
        /// Report the duplicate representation decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
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
    #[diagnostic(code = "EC603", message = "method must name its receiver explicitly")]
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

    /// Declaration repeats a member in the same owner.
    ///
    /// ```ds
    /// enum Status {
    ///     ready,
    ///     ready,
    /// }
    /// ```
    #[diagnostic(code = "EC612", message = "member '{member}' is already declared")]
    DuplicateMember {
        /// Report the later declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The repeated member name.
        member: String,
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

    /// Bodyless signature elides a result lifetime.
    ///
    /// ```ds
    /// declare function only(value: &Node): &Node;
    /// ```
    #[diagnostic(
        code = "EC614",
        message = "bodyless signatures must name result lifetimes explicitly"
    )]
    BodylessLifetimeElided {
        /// Report the bodyless declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Interface inheritance names a non-interface declaration.
    ///
    /// ```ds
    /// struct Shape {}
    /// interface Drawable extends Shape {}
    /// ```
    #[diagnostic(
        code = "EC615",
        message = "interface '{source}' can only extend interfaces, not '{target}'"
    )]
    InterfaceBaseNotInterface {
        /// Report the extends clause.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The extending interface.
        source: String,
        /// The invalid base type.
        target: String,
    },

    /// Implementation inheritance names a non-interface declaration.
    ///
    /// ```ds
    /// struct Shape {}
    /// struct Point implements Shape {}
    /// ```
    #[diagnostic(
        code = "EC616",
        message = "type '{source}' can only implement interfaces, not '{target}'"
    )]
    ImplementationTargetNotInterface {
        /// Report the implements clause.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The implementing type.
        source: String,
        /// The invalid implemented type.
        target: String,
    },

    /// Heritage reaches the same declaration with incompatible arguments.
    ///
    /// ```ds
    /// interface Base<T> {}
    /// interface Left extends Base<string> {}
    /// interface Right extends Base<int32> {}
    /// interface Both extends Left, Right {}
    /// ```
    #[diagnostic(
        code = "EC617",
        message = "type '{source}' has conflicting heritage for '{target}'"
    )]
    ConflictingHeritage {
        /// Report the conflicting heritage clause.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The declaration whose heritage is invalid.
        source: String,
        /// The repeated declaration.
        target: String,
    },

    /// Heritage reaches its own declaration again.
    ///
    /// ```ds
    /// interface A extends B {}
    /// interface B extends A {}
    /// ```
    #[diagnostic(code = "EC618", message = "type '{source}' has circular heritage")]
    CircularHeritage {
        /// Report the heritage clause whose branch exposes the cycle.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The declaration whose heritage is circular.
        source: String,
    },

    /// Exported nonlocal extension has no source name.
    ///
    /// ```ds
    /// export extension of External {}
    /// ```
    #[diagnostic(
        code = "EC619",
        message = "exported extension on nonlocal type '{target}' must have a name"
    )]
    UnnamedExportedNonlocalExtension {
        /// Report the exported extension.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The nonlocal extension target.
        target: String,
    },

    /// A written placement conflicts with the type's declared placement.
    ///
    /// ```ds
    /// shared class Registry {}
    ///
    /// declare const registry: local Registry;
    /// ```
    #[diagnostic(
        code = "EC620",
        message = "placement '{written}' conflicts with the declaration placement '{declared}'"
    )]
    PlacementConflict {
        /// Report the conflicting placement requirement.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written placement.
        written: String,
        /// The declared placement.
        declared: String,
    },

    /// A declaration's heritage requires inconsistent placements.
    ///
    /// ```ds
    /// local class Base {}
    /// shared interface Service {}
    /// class Invalid extends Base implements Service {}
    /// ```
    #[diagnostic(
        code = "EC621",
        message = "heritage declarations require one consistent placement"
    )]
    HeritagePlacementConflict {
        /// Report the conflicting heritage placement.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Enum variant value does not resolve to an integer or string constant.
    ///
    /// ```ds
    /// enum Status { Ready = true }
    /// ```
    #[diagnostic(
        code = "EC622",
        message = "enum variant value must be an integer or string constant, received '{ty}'"
    )]
    InvalidEnumVariantType {
        /// Report the invalid enum field.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected value type.
        ty: String,
    },

    /// Enum variants mix integer and string scalar domains.
    #[diagnostic(
        code = "EC623",
        message = "enum variants must all use the same scalar domain"
    )]
    MixedEnumVariantDomain {
        /// Report the conflicting enum variant.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// An implicit enum variant follows a string value.
    #[diagnostic(
        code = "EC624",
        message = "string backed enum variants require explicit values"
    )]
    ImplicitStringEnumVariant {
        /// Report the implicit enum variant.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Incrementing the preceding enum value overflows the integer domain.
    #[diagnostic(
        code = "EC625",
        message = "implicit enum variant value overflows int64"
    )]
    EnumVariantValueOverflow {
        /// Report the implicit enum variant.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 7xx: diagnostics
    // -------------------------------------------------------------------------
    /// An expected compiler diagnostic did not occur.
    #[diagnostic(
        code = "EC700",
        message = "expected diagnostic '{selector}' did not occur"
    )]
    UnmetDiagnosticExpectation {
        /// Report the expectation selector.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The expected diagnostic selector.
        selector: String,
    },

    /// A diagnostic control overrides an enclosing forbid.
    ///
    /// ```ds
    /// @forbid("WC402")
    /// @allow("WC402")
    /// if (true) {}
    /// ```
    #[diagnostic(
        code = "EC701",
        message = "diagnostic '{selector}' is forbidden by an enclosing control"
    )]
    ForbiddenDiagnosticOverride {
        /// Report the rejected diagnostic selector.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The forbidden diagnostic selector.
        selector: String,
    },
}
