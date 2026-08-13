use crate::DiagnosticAnchor;
use destack_artifact::{DiagnosticError, DiagnosticFormat, DiagnosticFormatter};
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// The signature an object type declares beside its named properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixedObjectSignature {
    /// An index signature.
    IndexSignature,
    /// A call signature.
    CallSignature,
    /// A construct signature.
    ConstructSignature,
}

impl DiagnosticFormat for MixedObjectSignature {
    /// Name the conflicting signature in a diagnostic message.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(match self {
            Self::IndexSignature => "an index signature".to_string(),
            Self::CallSignature => "a call signature".to_string(),
            Self::ConstructSignature => "a construct signature".to_string(),
        })
    }
}

/// Errors during the check phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Check)]
pub enum CheckError {
    // -------------------------------------------------------------------------
    // inference
    // -------------------------------------------------------------------------
    /// Inference could not determine a required type or static value.
    ///
    /// ```ds
    /// const value = _;
    /// ```
    #[diagnostic(
        id = "cannot-infer-type",
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
    #[diagnostic(id = "missing-type-annotation", message = "missing type annotation")]
    MissingTypeAnnotation {
        /// Report the declaration that needs a type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Export's type depends on another module and needs an annotation.
    ///
    /// ```ds
    /// export const value = imported();
    /// ```
    #[diagnostic(
        id = "export-type-not-module-derivable",
        message = "export's type is not derivable within its module",
        help = "annotate the exported declaration"
    )]
    ExportTypeNotDerivable {
        /// Report the declaration whose type needs another module.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Named function requires a written result type.
    ///
    /// ```ds
    /// function scale(value: float64) { }
    /// ```
    #[diagnostic(
        id = "missing-result-type",
        message = "function declaration needs a written result type",
        help = "state the result type on the declaration"
    )]
    MissingResultType {
        /// Report the function declaration without a result type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Exported binding requires a written type.
    ///
    /// ```ds
    /// export const value = compute();
    /// ```
    #[diagnostic(
        id = "missing-export-binding-type",
        message = "exported binding needs a written type",
        help = "state the type or initialize with a literal"
    )]
    MissingExportBindingType {
        /// Report the exported binding without a written type.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Transparent type expansion reached the same type again.
    ///
    /// ```ds
    /// type Loop = Loop;
    /// ```
    #[diagnostic(id = "circular-type", message = "type is circular")]
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
    #[diagnostic(id = "unsupported-source-type", message = "unsupported type: {name}")]
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
        id = "wrong-generic-arity",
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
        id = "invalid-type-query",
        message = "typeof type query requires a value reference"
    )]
    InvalidTypeQuery {
        /// Report the invalid query operand.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // relations
    // -------------------------------------------------------------------------
    /// Source type is not assignable to target type.
    ///
    /// ```ds
    /// let value: string = 1;
    /// ```
    #[diagnostic(
        id = "not-assignable",
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

    /// Source type converts to more than one represented union case.
    ///
    /// ```ds
    /// type Value = { x: int32 } | { x: int32; y?: int32 };
    /// declare const source: { x: int32; y: int32 };
    /// const value: Value = source;
    /// ```
    #[diagnostic(
        id = "ambiguous-union-coercion",
        message = "type '{source}' converts to multiple cases of union '{target}'",
        help = "cast the value to one union member before assigning it"
    )]
    AmbiguousUnionCoercion {
        /// Report the converted source value.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The converted source type.
        source: String,
        /// The represented target union.
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
        id = "constraint-not-satisfied",
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

    /// Strict equality compares identity, so its operands must carry one.
    ///
    /// ```ds
    /// struct Point {
    ///     x: int32;
    /// }
    ///
    /// declare const a: Point;
    /// declare const b: Point;
    ///
    /// const same = a === b;
    /// ```
    #[diagnostic(
        id = "no-strict-identity",
        message = "value type '{ty}' has no identity, compare with '=='"
    )]
    NoStrictIdentity {
        /// Report the strict comparison.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The compared value type.
        ty: String,
    },

    /// An equality requirement has unequal normalized operands.
    ///
    /// ```ds
    /// function same<T, U>(): void where T == U {}
    ///
    /// same<int32, string>();
    /// ```
    #[diagnostic(
        id = "equality-requirement-not-satisfied",
        message = "equality requirement '{left} == {right}' is not satisfied"
    )]
    EqualityRequirementNotSatisfied {
        /// Report the failed equality relation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The normalized left operand.
        left: String,
        /// The normalized right operand.
        right: String,
    },

    /// Type does not extend a required base type.
    ///
    /// ```ds
    /// class User extends number {}
    /// ```
    #[diagnostic(
        id = "does-not-extend",
        message = "type '{source}' does not extend '{target}'"
    )]
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
        id = "interface-not-implemented",
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
        id = "non-writable-assignment-target",
        message = "assignment target is not a writable place"
    )]
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
        id = "excess-property",
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

    /// Struct literal contains a getter or setter instead of a field initializer.
    ///
    /// ```ds
    /// struct Store {
    ///     value: () => string;
    /// }
    ///
    /// Store { get value(): string { return "ready"; } };
    /// ```
    #[diagnostic(
        id = "invalid-struct-accessor",
        message = "accessors are not valid in struct literals"
    )]
    InvalidStructAccessor {
        /// Report the invalid accessor property.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Type cannot be explicitly cast to the requested target type.
    ///
    /// ```ds
    /// const value = "text" as int32;
    /// ```
    #[diagnostic(
        id = "invalid-cast",
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
    #[diagnostic(
        id = "invalid-intrinsic-type",
        message = "intrinsic type is not valid here"
    )]
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
    #[diagnostic(id = "invalid-const-type", message = "const type is not valid here")]
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
        id = "argument-not-assignable",
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
        id = "return-not-assignable",
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
        id = "spread-not-object",
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
        id = "cannot-assign-immutable-binding",
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
    #[diagnostic(
        id = "cannot-assign-imported-binding",
        message = "cannot assign to imported binding '{name}'"
    )]
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
        id = "cannot-assign-readonly-member",
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

    /// Assignment writes a computed key through a structural index signature.
    ///
    /// ```ds
    /// declare const counts: { [key: string]: int32 };
    /// declare const key: string;
    ///
    /// counts[key] = 1;
    /// ```
    #[diagnostic(
        id = "cannot-assign-structural-index",
        message = "cannot assign a computed key through the structural type '{receiver}', type the receiver as an IndexSet implementer like Map"
    )]
    CannotAssignStructuralIndex {
        /// Report the mutation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The structural receiver type.
        receiver: String,
    },

    /// Assigned object is missing a required property.
    ///
    /// ```ds
    /// const value: { name: string } = {};
    /// ```
    #[diagnostic(
        id = "missing-required-property",
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
    /// declare const point: Point;
    ///
    /// write(point);
    /// ```
    #[diagnostic(
        id = "writable-index-requires-index-set",
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
        id = "borrow-access-not-granted",
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
    // selection
    // -------------------------------------------------------------------------
    /// Receiver type does not contain a selected member.
    ///
    /// ```ds
    /// declare const user: { name: string };
    ///
    /// user.missing;
    /// ```
    #[diagnostic(
        id = "missing-member",
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
    #[diagnostic(id = "not-callable", message = "value of type '{ty}' is not callable")]
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
        id = "no-matching-call",
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
        id = "no-matching-construct",
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
    #[diagnostic(
        id = "invalid-derive-provider",
        message = "derive argument must name a registered provider newtype"
    )]
    InvalidDeriveProvider {
        /// Report the derive provider argument.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A derive provider does not support the annotated declaration.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// struct Shape {}
    /// ```
    #[diagnostic(
        id = "invalid-derive-target",
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
    #[diagnostic(
        id = "duplicate-derive-provider",
        message = "duplicate derive provider '{provider}'"
    )]
    DuplicateDeriveProvider {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The duplicated derive provider.
        provider: String,
    },

    /// One Tagged backing arm lies outside the constructible record domain.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = string;
    /// ```
    #[diagnostic(
        id = "invalid-tagged-variant",
        message = "Tagged backing arm must be a constructible shape or struct"
    )]
    InvalidTaggedVariant {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A Tagged backing has no inferable discriminator.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = { value: int32 };
    /// ```
    #[diagnostic(
        id = "missing-tagged-discriminator",
        message = "Tagged backing has no common required field with distinct string literal types"
    )]
    MissingTaggedDiscriminator {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// A Tagged backing has more than one inferable discriminator.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype State = { kind: "on"; state: "ready" } | { kind: "off"; state: "done" };
    /// ```
    #[diagnostic(
        id = "ambiguous-tagged-discriminator",
        message = "Tagged backing has multiple possible discriminators: {discriminators}"
    )]
    AmbiguousTaggedDiscriminator {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The possible discriminator fields.
        discriminators: String,
    },

    /// An explicitly selected Tagged discriminator is invalid.
    ///
    /// ```ds
    /// @derive(Tagged({ discriminator: "type" }))
    /// newtype Shape = { kind: "circle" };
    /// ```
    #[diagnostic(
        id = "invalid-tagged-discriminator",
        message = "Tagged discriminator '{discriminator}' must be a required string literal field in every backing arm"
    )]
    InvalidTaggedDiscriminator {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected discriminator field.
        discriminator: String,
    },

    /// Two Tagged backing arms carry the same discriminant.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = { kind: "shape" } | { kind: "shape"; radius: float64 };
    /// ```
    #[diagnostic(
        id = "duplicate-tagged-discriminant",
        message = "duplicate Tagged discriminant '{discriminant}'"
    )]
    DuplicateTaggedDiscriminant {
        /// Report the derive application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The duplicated discriminant value.
        discriminant: String,
    },

    /// A Tagged discriminant cannot produce a declaration member name.
    ///
    /// ```ds
    /// @derive(Tagged)
    /// newtype Shape = { kind: "---" };
    /// ```
    #[diagnostic(
        id = "invalid-tagged-case",
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
    #[diagnostic(
        id = "duplicate-tagged-case",
        message = "duplicate Tagged case '{key}'"
    )]
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
    #[diagnostic(id = "ambiguous-member", message = "member '{key}' is ambiguous")]
    AmbiguousMember {
        /// Report the member access.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The selected member key.
        key: String,
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
    #[diagnostic(id = "inaccessible-member", message = "member '{key}' is {visibility}")]
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
        id = "no-matching-operator",
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
        id = "invalid-strict-equality",
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
        id = "unresolved-reference",
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
    #[diagnostic(id = "ambiguous-reference", message = "ambiguous reference '{name}'")]
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
    #[diagnostic(
        id = "invalid-decorator-target",
        message = "decorator must name a newtype declaration"
    )]
    InvalidDecoratorTarget {
        /// Report the decorator target expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Decorator arguments do not select exactly one backing alternative.
    ///
    /// ```ds
    /// newtype mark = (string,) | (`${string}`,);
    ///
    /// @mark("value")
    /// const value = 1;
    /// ```
    #[diagnostic(
        id = "ambiguous-decorator",
        message = "decorator arguments must select exactly one newtype backing"
    )]
    AmbiguousDecorator {
        /// Report the decorator application.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Decorator arguments match no backing alternative.
    ///
    /// ```ds
    /// newtype mark = (string,) | (boolean,);
    ///
    /// @mark(1)
    /// const value = 1;
    /// ```
    #[diagnostic(
        id = "no-matching-decorator",
        message = "decorator arguments do not match any newtype backing"
    )]
    NoMatchingDecorator {
        /// Report the decorator application.
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
    #[diagnostic(
        id = "duplicate-capture-decorator",
        message = "duplicate capture decorator"
    )]
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
        id = "invalid-capture-target",
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
        id = "possibly-nullish",
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
        id = "not-constructible",
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
        id = "invalid-inferred-construct-target",
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
        id = "wrong-argument-count",
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
    #[diagnostic(
        id = "invalid-index-receiver",
        message = "type '{receiver}' cannot be indexed"
    )]
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
        id = "invalid-index-key",
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
        id = "instance-of-target-not-class",
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
        id = "impossible-instance-of",
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
        id = "impossible-is",
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
        id = "runtime-predicate-not-testable",
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
    #[diagnostic(
        id = "cannot-read-write-only-member",
        message = "member '{member}' is write-only"
    )]
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
        id = "receiver-not-assignable",
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
    // expressions
    // -------------------------------------------------------------------------
    /// Runtime condition does not have boolean type.
    ///
    /// ```ds
    /// if (1) {}
    /// ```
    #[diagnostic(
        id = "non-boolean-condition",
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
        id = "invalid-static-condition",
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
        id = "undecidable-static-condition",
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
    #[diagnostic(
        id = "undecidable-static-value",
        message = "static value must be statically decidable"
    )]
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
    #[diagnostic(
        id = "invalid-static-if-invocation",
        message = "`@if` must be invoked as `@if(condition)`"
    )]
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
    #[diagnostic(
        id = "break-outside-control-target",
        message = "break statement has no target"
    )]
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
        id = "break-value-outside-loop",
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
    #[diagnostic(
        id = "unused-generic-parameter",
        message = "generic parameter '{name}' is never used"
    )]
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
        id = "variance-conflict",
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
        id = "non-exhaustive-pattern",
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
    #[diagnostic(
        id = "use-before-assigned",
        message = "'{name}' is used before being assigned"
    )]
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
        id = "refutable-pattern",
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
    #[diagnostic(
        id = "await-outside-async-context",
        message = "await expression requires an async context"
    )]
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
    #[diagnostic(
        id = "yield-outside-generator",
        message = "yield expression requires a generator"
    )]
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
    #[diagnostic(
        id = "invalid-static-operation",
        message = "static evaluation failed: {message}"
    )]
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
        id = "invalid-try-operand",
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
    #[diagnostic(
        id = "invalid-pattern-tag",
        message = "pattern tag '{ty}' is not a nominal type"
    )]
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
    #[diagnostic(
        id = "continue-outside-loop",
        message = "continue statement has no target"
    )]
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
    #[diagnostic(
        id = "return-outside-function",
        message = "return statement is outside a function"
    )]
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
    #[diagnostic(id = "this-outside-receiver", message = "'this' is not available here")]
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
    #[diagnostic(id = "super-outside-class", message = "'super' is not available here")]
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
    #[diagnostic(
        id = "let-else-branch-can-complete",
        message = "else branch of let-else must diverge"
    )]
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
        id = "missing-tree-builder",
        message = "tree expression requires an active tree builder"
    )]
    MissingTreeBuilder {
        /// Report the tree expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Tree tag is not declared by the builder's rows.
    ///
    /// ```ds
    /// const page: Html = <blink/>;
    /// ```
    #[diagnostic(
        id = "unknown-tree-tag",
        message = "builder '{builder}' declares no '{tag}' tag"
    )]
    UnknownTreeTag {
        /// Report the tree expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written tag.
        tag: String,
        /// The resolved builder.
        builder: String,
    },

    /// Tree attribute outside the declared attribute row.
    ///
    /// ```ds
    /// const page: Html = <div misspelled="1"/>;
    /// ```
    #[diagnostic(
        id = "unknown-tree-attribute",
        message = "attribute row '{row}' declares no '{key}' attribute"
    )]
    UnknownTreeAttribute {
        /// Report the tree expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The written attribute key.
        key: String,
        /// The declared attribute row.
        row: String,
    },

    /// Tree spread child over a dynamically sized operand.
    ///
    /// ```ds
    /// const page: Html = <div>{...items}</div>;
    /// ```
    #[diagnostic(
        id = "tree-spread-not-tuple",
        message = "spread children splat tuples, found '{ty}'"
    )]
    TreeSpreadNotTuple {
        /// Report the spread operand.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The spread operand type.
        ty: String,
    },

    /// Tree expression missing one required attribute.
    ///
    /// ```ds
    /// const page: Html = <img/>;
    /// ```
    #[diagnostic(
        id = "missing-tree-attribute",
        message = "required attribute '{key}' of row '{row}' is missing"
    )]
    MissingTreeAttribute {
        /// Report the tree expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The missing attribute key.
        key: String,
        /// The declared attribute row.
        row: String,
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
    #[diagnostic(
        id = "expression-pattern-not-literal",
        message = "expression pattern must close to a literal"
    )]
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
    #[diagnostic(
        id = "yield-delegate-missing-value",
        message = "yield* expression requires a value"
    )]
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
        id = "try-outside-function",
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
    #[diagnostic(
        id = "for-of-source-not-iterable",
        message = "for-of source must be iterable"
    )]
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
    #[diagnostic(
        id = "for-in-source-not-object-shaped",
        message = "for-in source must be object-shaped"
    )]
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
        id = "pattern-source-not-object-shaped",
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
        id = "pattern-source-not-tuple-shaped",
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
        id = "pattern-source-not-sequence-shaped",
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
        id = "pattern-field-missing",
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
        id = "pattern-member-not-field",
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
        id = "duplicate-pattern-field",
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
        id = "duplicate-pattern-binding",
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
    #[diagnostic(id = "rest-pattern-not-last", message = "rest pattern must be last")]
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
    #[diagnostic(
        id = "multiple-rest-patterns",
        message = "pattern can contain at most one rest field"
    )]
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
        id = "computed-pattern-key-not-valid",
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
    #[diagnostic(
        id = "invalid-range-pattern-domain",
        message = "range pattern cannot match type '{domain}'"
    )]
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
        id = "invalid-range-pattern-bound",
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
        id = "pattern-alternative-binding-mismatch",
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
        id = "destructuring-assignment-requires-plain-assignment",
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
        id = "refutable-catch-pattern",
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
        id = "pattern-variant-not-in-type",
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
        id = "pattern-variant-missing",
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
    // representation
    // -------------------------------------------------------------------------
    /// Type is not concrete and therefore has no layout.
    ///
    /// ```ds
    /// function size<T>(): usize {
    ///     return comptime sizeOf<T>();
    /// }
    /// ```
    #[diagnostic(
        id = "layout-not-concrete",
        message = "type '{ty}' has no concrete layout"
    )]
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
        id = "unsupported-representation",
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
    #[diagnostic(
        id = "unbounded-interval-type",
        message = "interval type must be bounded"
    )]
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
        id = "invalid-interval-domain",
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
    #[diagnostic(
        id = "dynamic-safety-not-satisfied",
        message = "type '{ty}' is not dynamic-safe"
    )]
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
        id = "overwrite-stability-not-satisfied",
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
        id = "local-reference-in-shared-storage",
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
        id = "non-integer-c-enum",
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
        id = "enum-value-outside-representation",
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
    #[diagnostic(
        id = "duplicate-representation-decorator",
        message = "duplicate representation decorator"
    )]
    DuplicateRepresentationDecorator {
        /// Report the duplicate representation decorator.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // declarations
    // -------------------------------------------------------------------------
    /// Override declaration does not match an inherited member.
    ///
    /// ```ds
    /// class User {
    ///     override name() {}
    /// }
    /// ```
    #[diagnostic(
        id = "invalid-override",
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
        id = "unimplemented-abstract-member",
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
        id = "cannot-construct-abstract-type",
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
    #[diagnostic(
        id = "missing-explicit-receiver",
        message = "method must name its receiver explicitly"
    )]
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
        id = "conflicting-implementation",
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
        id = "foreign-blanket-implementation",
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
        id = "missing-override",
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
        id = "override-not-virtual",
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
    #[diagnostic(
        id = "final-class-extended",
        message = "final class '{ty}' cannot be extended"
    )]
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
        id = "abstract-member-in-concrete-class",
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
        id = "incompatible-override",
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
    #[diagnostic(
        id = "missing-declaration-body",
        message = "declaration '{name}' requires a body"
    )]
    MissingDeclarationBody {
        /// Report the bodyless declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The bodyless declaration name.
        name: String,
    },

    /// One lifetime bound spelled as a union of lifetimes.
    ///
    /// ```ds
    /// function hold<'a, 'b>(value: &'a int32) where 'a: 'a | 'b {}
    /// ```
    #[diagnostic(
        id = "disjunctive-lifetime-bound",
        message = "a lifetime bound must name one lifetime, not a union"
    )]
    DisjunctiveLifetimeBound {
        /// Report the where clause.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Declaration repeats a member in the same owner.
    ///
    /// ```ds
    /// enum Status {
    ///     ready,
    ///     ready,
    /// }
    /// ```
    #[diagnostic(
        id = "duplicate-member",
        message = "member '{member}' is already declared",
        optional_message = " for '{target}' by another visible extension"
    )]
    DuplicateMember {
        /// Report the later declaration.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The repeated member name.
        member: String,
        /// The extended target when another extension owns the slot.
        target: Option<String>,
    },

    /// Object type declares named properties beside an index or call signature.
    ///
    /// ```ds
    /// type Row = { name: string; [key: string]: string };
    /// ```
    #[diagnostic(
        id = "mixed-object-type",
        message = "object type mixes named properties with {conflict}"
    )]
    MixedObjectType {
        /// Report the object type expression.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The signature that conflicts with the named properties.
        conflict: MixedObjectSignature,
    },

    /// Class field is not definitely initialized.
    ///
    /// ```ds
    /// class User {
    ///     name: string;
    /// }
    /// ```
    #[diagnostic(
        id = "field-not-definitely-initialized",
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

    /// Interface inheritance names a non-interface declaration.
    ///
    /// ```ds
    /// struct Shape {}
    /// interface Drawable extends Shape {}
    /// ```
    #[diagnostic(
        id = "interface-base-not-interface",
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
        id = "implementation-target-not-interface",
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
        id = "conflicting-heritage",
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
    #[diagnostic(
        id = "circular-heritage",
        message = "type '{source}' has circular heritage"
    )]
    CircularHeritage {
        /// Report the heritage clause whose branch exposes the cycle.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The declaration whose heritage is circular.
        source: String,
    },

    /// Elided lifetime inside a type declaration.
    ///
    /// ```ds
    /// struct Entry { name: &string }
    /// ```
    #[diagnostic(
        id = "elided-declaration-lifetime",
        message = "type declaration '{source}' writes its lifetimes",
        help = "declare the lifetime parameter and name it, like &'a"
    )]
    ElidedLifetimeInNamedDeclaration {
        /// Report the elided borrow position.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The declaration that names its lifetimes.
        source: String,
    },

    /// Exported nonlocal extension has no source name.
    ///
    /// ```ds
    /// export extension of External {}
    /// ```
    #[diagnostic(
        id = "unnamed-exported-nonlocal-extension",
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
        id = "placement-conflict",
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
        id = "heritage-placement-conflict",
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
        id = "invalid-enum-variant-type",
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
        id = "mixed-enum-variant-domain",
        message = "enum variants must all use the same scalar domain"
    )]
    MixedEnumVariantDomain {
        /// Report the conflicting enum variant.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    /// Two enum variants use the same runtime value.
    #[diagnostic(
        id = "duplicate-enum-variant-value",
        message = "enum variant value '{value}' is already declared"
    )]
    DuplicateEnumVariantValue {
        /// Report the repeated enum value.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The repeated scalar value.
        value: String,
    },

    /// An implicit enum variant follows a string value.
    #[diagnostic(
        id = "implicit-string-enum-variant",
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
        id = "enum-variant-value-overflow",
        message = "implicit enum variant value overflows int64"
    )]
    EnumVariantValueOverflow {
        /// Report the implicit enum variant.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // diagnostics
    // -------------------------------------------------------------------------
    /// An expected compiler diagnostic did not occur.
    #[diagnostic(
        id = "unmet-diagnostic-expectation",
        message = "expected diagnostic '{diagnostic}' did not occur"
    )]
    UnmetDiagnosticExpectation {
        /// Report the diagnostic expectation.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The expected diagnostic id.
        diagnostic: String,
    },

    /// A diagnostic control overrides an enclosing forbid.
    ///
    /// ```ds
    /// @forbid("constant-condition")
    /// @allow("constant-condition")
    /// if (true) {}
    /// ```
    #[diagnostic(
        id = "forbidden-diagnostic-override",
        message = "diagnostic '{diagnostic}' is forbidden by an enclosing control"
    )]
    ForbiddenDiagnosticOverride {
        /// Report the rejected diagnostic id.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The forbidden diagnostic id.
        diagnostic: String,
    },

    /// A diagnostic control names an unknown id.
    #[diagnostic(
        id = "unknown-diagnostic",
        message = "unknown diagnostic '{diagnostic}'"
    )]
    UnknownDiagnostic {
        /// Report the rejected diagnostic id.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The unknown diagnostic id.
        diagnostic: String,
    },

    /// A diagnostic control names an uncontrollable diagnostic.
    #[diagnostic(
        id = "uncontrollable-diagnostic",
        message = "diagnostic '{diagnostic}' cannot be controlled"
    )]
    UncontrollableDiagnostic {
        /// Report the rejected diagnostic id.
        anchor: DiagnosticAnchor,
        /// The module being checked.
        module: ModuleId,
        /// The rejected canonical diagnostic id.
        diagnostic: String,
    },
}
