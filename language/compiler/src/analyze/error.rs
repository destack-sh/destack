use crate::{
    CompileError, DiagnosticAnchor, DiagnosticDefinition, RequirementError, RequirementSet,
    ResolveError,
};
use destack_compiler_macros::DefineError;
use destack_core::StringId;
use destack_dir::{AnchoredGlobalNodeId, GlobalSymbolId, GlobalTypeId, StaticKey, Visibility};
use destack_source::ModuleId;
use destack_workspace::Repository;

/// The diagnostic abstraction state of one callable member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallableAbstraction {
    /// An abstract callable.
    Abstract,
    /// An abstract override callable.
    AbstractOverride,
    /// A concrete override callable.
    Override,
    /// A concrete non-override callable.
    Concrete,
}

/// Errors during the analyze phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Analyze)]
pub enum AnalyzeError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EA000", r#yield)]
    Yield { requirement: RequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EA001", yield_failed)]
    UnsatisfiedRequirement { requirement: RequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "EA002", message = "task skipped")]
    Skipped,

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
    #[error(
        code = "EA102",
        message = "expected {expected_ty}, found {actual_ty} (not assignable)"
    )]
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

    /// Enum member has an invalid value.
    #[error(code = "EA105", message = "invalid enum field value")]
    InvalidEnumFieldValue { node: AnchoredGlobalNodeId },

    /// Strict equality requires identity types.
    #[error(
        code = "EA106",
        message = "strict equality not supported for non-identity type {ty}"
    )]
    InvalidStrictEquality {
        node: AnchoredGlobalNodeId,
        ty: GlobalTypeId,
    },

    /// Implicit any type.
    #[error(code = "EA107", message = "implicit any type")]
    ImplicitAny { node: AnchoredGlobalNodeId },

    /// Implicit this type.
    #[error(code = "EA108", message = "implicit this type")]
    ImplicitThis { node: AnchoredGlobalNodeId },

    /// Static value arguments must be static expressions.
    #[error(
        code = "EA109",
        message = "static argument must be a static expression"
    )]
    NonStaticArgument { node: AnchoredGlobalNodeId },

    /// Ownership operators require an unowned value.
    #[error(
        code = "EA110",
        message = "ownership operator requires an unowned value, found {actual_ty}"
    )]
    InvalidOwnershipOperand {
        node: AnchoredGlobalNodeId,
        actual_ty: GlobalTypeId,
    },

    /// Array size expressions must be constant integers.
    #[error(code = "EA111", message = "array size must be a constant integer")]
    InvalidArraySize { node: AnchoredGlobalNodeId },

    /// Static arguments cannot form a cycle.
    #[error(code = "EA112", message = "static argument cycle")]
    CircularStaticArgument { node: AnchoredGlobalNodeId },

    /// Array literals cannot contain holes.
    #[error(code = "EA113", message = "array literal holes are not allowed")]
    ArrayLiteralHole { node: AnchoredGlobalNodeId },

    /// Static argument is required but was not provided.
    #[error(code = "EA114", message = "missing static argument")]
    MissingStaticArgument { node: AnchoredGlobalNodeId },

    /// Static value parameters must be marked with comptime.
    #[error(
        code = "EA115",
        message = "static value parameters must be explicitly marked with comptime"
    )]
    StaticParameterRequiresComptime { node: AnchoredGlobalNodeId },

    /// Interface inference cycle requires an explicit annotation.
    #[error(
        code = "EA116",
        message = "export requires annotation to break inference cycle"
    )]
    InterfaceInferenceRequiresAnnotation { node: AnchoredGlobalNodeId },

    /// Type only symbols cannot be used as values.
    #[error(code = "EA117", message = "type-only symbol cannot be used as a value")]
    TypeOnlyValue { node: AnchoredGlobalNodeId },

    /// Cannot assign to a readonly property.
    #[error(
        code = "EA118",
        message = "cannot assign to readonly property {member_key}"
    )]
    ReadonlyProperty {
        node: AnchoredGlobalNodeId,
        member_key: StaticKey,
    },

    /// Cannot assign to an immutable binding.
    #[error(code = "EA119", message = "cannot assign to immutable binding")]
    ImmutableBindingAssignment { node: AnchoredGlobalNodeId },

    /// Cannot assign through an immutable reference.
    #[error(code = "EA120", message = "cannot assign through immutable reference")]
    ImmutableReferenceAssignment { node: AnchoredGlobalNodeId },

    /// Recursive type instantiation.
    #[error(code = "EA121", message = "recursive type instantiation")]
    RecursiveTypeInstantiation { node: AnchoredGlobalNodeId },

    /// Comptime expressions must be static expressions.
    #[error(
        code = "EA122",
        message = "comptime expression must be a static expression"
    )]
    InvalidComptimeExpression { node: AnchoredGlobalNodeId },

    /// Void is not allowed in tuple types.
    #[error(code = "EA123", message = "void is not allowed in tuples")]
    VoidInTuple { node: AnchoredGlobalNodeId },

    /// Void is not allowed in array types.
    #[error(code = "EA124", message = "void is not allowed in arrays")]
    VoidInArray { node: AnchoredGlobalNodeId },

    /// Invalid static argument.
    #[error(code = "EA125", message = "invalid static argument: {message}")]
    InvalidStaticArgument {
        /// Report the static argument context node.
        node: AnchoredGlobalNodeId,
        /// Describe why the static argument is invalid.
        message: String,
    },

    /// Infer declarations are only valid in conditional type extends clauses.
    #[error(
        code = "EA126",
        message = "infer declarations are only permitted in the extends clause of a conditional type"
    )]
    InferOutsideConditional { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 2xx: Callable / member / operator / object errors
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

    /// Call argument count does not satisfy the required arity.
    #[error(
        code = "EA236",
        message = "expected {expected} arguments, found {actual}"
    )]
    InvalidArgumentArity {
        node: AnchoredGlobalNodeId,
        expected: usize,
        actual: usize,
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

    /// Invalid instanceof target.
    #[error(code = "EA210", message = "instanceof requires a class type")]
    InvalidInstanceOfTarget { node: AnchoredGlobalNodeId },

    /// Duplicate overload signature in a non-declaration module.
    #[error(code = "EA211", message = "duplicate overload signature")]
    DuplicateOverloadSignature { node: AnchoredGlobalNodeId },

    /// Multiple overload implementations in a TypeScript module.
    #[error(code = "EA212", message = "multiple overload implementations")]
    MultipleOverloadImplementations { node: AnchoredGlobalNodeId },

    /// Multiple constructor implementations in a TypeScript module.
    #[error(code = "EA213", message = "multiple constructor implementations")]
    MultipleConstructorImplementations { node: AnchoredGlobalNodeId },

    /// Reserved identifier used as a binding name.
    #[error(code = "EA214", message = "reserved identifier '{name}'")]
    ReservedIdentifier {
        node: AnchoredGlobalNodeId,
        name: StringId,
    },

    /// Object literal property defaults are not allowed.
    #[error(
        code = "EA215",
        message = "object literal property defaults are not allowed"
    )]
    ObjectLiteralDefault { node: AnchoredGlobalNodeId },

    /// Object pattern can only contain one spread field.
    #[error(code = "EA216", message = "object pattern can only contain one spread")]
    ObjectPatternMultipleSpreads { node: AnchoredGlobalNodeId },

    /// Object pattern spread must be the last field.
    #[error(code = "EA217", message = "object pattern spread must be last")]
    ObjectPatternSpreadNotLast { node: AnchoredGlobalNodeId },

    /// Export namespace is only allowed in declaration modules.
    #[error(
        code = "EA218",
        message = "export as namespace is only allowed in declaration modules"
    )]
    ExportNamespaceOutsideDeclaration { node: AnchoredGlobalNodeId },

    /// Invalid type parameter modifier.
    #[error(code = "EA219", message = "invalid type parameter modifier")]
    InvalidTypeParameterModifier { node: AnchoredGlobalNodeId },

    /// Readonly type must target an array or tuple.
    #[error(
        code = "EA220",
        message = "readonly type must target an array or tuple"
    )]
    InvalidReadonlyType { node: AnchoredGlobalNodeId },

    /// Optional tuple elements must be last.
    #[error(code = "EA221", message = "optional tuple elements must be last")]
    InvalidTupleElementOrder { node: AnchoredGlobalNodeId },

    /// Intrinsic types cannot be indexed.
    #[error(code = "EA222", message = "intrinsic types cannot be indexed")]
    InvalidIntrinsicTypeIndex { node: AnchoredGlobalNodeId },

    /// Destructuring declarations require initializers.
    #[error(
        code = "EA223",
        message = "destructuring declarations require initializers"
    )]
    MissingDestructuringInitializer { node: AnchoredGlobalNodeId },

    /// Declare bindings cannot have initializers.
    #[error(code = "EA224", message = "declare bindings cannot have initializers")]
    InvalidDeclareInitializer { node: AnchoredGlobalNodeId },

    /// Type-only imports cannot mix default and named bindings.
    #[error(
        code = "EA225",
        message = "type-only imports cannot mix default and named bindings"
    )]
    InvalidTypeOnlyImportBindings { node: AnchoredGlobalNodeId },

    /// Assignment targets must be assignable expressions.
    #[error(code = "EA226", message = "invalid assignment target")]
    InvalidAssignmentTarget { node: AnchoredGlobalNodeId },

    /// Static constraint resolution requires a static parameter symbol.
    #[error(code = "EA227", message = "invalid static constraint target")]
    InvalidStaticConstraint { node: AnchoredGlobalNodeId },

    /// TypeScript syntax is not allowed in JavaScript modules.
    #[error(
        code = "EA228",
        message = "typescript syntax is not allowed in modules"
    )]
    TypeScriptSyntaxInJavaScript { node: AnchoredGlobalNodeId },

    /// Import aliases cannot use `import type`.
    #[error(code = "EA229", message = "import aliases cannot use 'import type'")]
    InvalidTypeOnlyImportAlias { node: AnchoredGlobalNodeId },

    /// Const declarations require initializers.
    #[error(code = "EA230", message = "const declarations require initializers")]
    MissingConstInitializer { node: AnchoredGlobalNodeId },

    /// Const initializers in ambient contexts must be literal values or enum references.
    #[error(
        code = "EA231",
        message = "const initializers in ambient contexts must be literal values or enum references"
    )]
    InvalidAmbientConstInitializer { node: AnchoredGlobalNodeId },

    /// Definite assignment assertions are not valid in variable declarators.
    #[error(
        code = "EA232",
        message = "definite assignment assertions are not valid in variable declarators"
    )]
    InvalidDefiniteAssignmentDeclarator { node: AnchoredGlobalNodeId },

    /// Import aliases must target a qualified identifier path.
    #[error(
        code = "EA233",
        message = "import aliases must target a qualified identifier path"
    )]
    InvalidImportAliasTarget { node: AnchoredGlobalNodeId },

    /// Member-like access cannot directly follow instantiation expressions.
    #[error(
        code = "EA234",
        message = "instantiation expressions must be parenthesized before member or index access"
    )]
    InvalidInstantiationAccess { node: AnchoredGlobalNodeId },

    /// Object pattern rest must be an identifier.
    #[error(code = "EA235", message = "object pattern rest must be an identifier")]
    ObjectPatternRestNotIdentifier { node: AnchoredGlobalNodeId },

    /// Duplicate default export in a module.
    #[error(code = "EA238", message = "duplicate default export")]
    DuplicateDefaultExport {
        node: AnchoredGlobalNodeId,
        other_node: AnchoredGlobalNodeId,
    },

    /// Private identifiers must be used in `#name in object` expressions.
    #[error(
        code = "EA239",
        message = "private identifiers must appear in 'in' expressions"
    )]
    InvalidPrivateIdentifier { node: AnchoredGlobalNodeId },

    /// Exponentiation cannot take an unparenthesized unary expression on the left.
    #[error(
        code = "EA240",
        message = "unparenthesized unary expression cannot be the left operand of '**'"
    )]
    InvalidExponentLeftUnary { node: AnchoredGlobalNodeId },

    /// Parenthesized expressions cannot be empty in JS/TS.
    #[error(code = "EA241", message = "empty parenthesized expression")]
    EmptyParenthesizedExpression { node: AnchoredGlobalNodeId },

    /// Tagged templates cannot be applied to optional chains.
    #[error(
        code = "EA242",
        message = "tagged templates cannot follow optional chains"
    )]
    InvalidOptionalChainTemplate { node: AnchoredGlobalNodeId },

    /// Duplicate labels are not allowed in nested label scopes.
    #[error(code = "EA243", message = "duplicate statement label")]
    DuplicateLabel { node: AnchoredGlobalNodeId },

    /// Super calls are only valid in constructors of derived classes.
    #[error(
        code = "EA244",
        message = "super calls are only valid in constructors of derived classes"
    )]
    InvalidSuperCall { node: AnchoredGlobalNodeId },

    /// Optional chaining cannot be applied to super.
    #[error(
        code = "EA245",
        message = "optional chaining cannot be applied to super"
    )]
    InvalidSuperOptionalChain { node: AnchoredGlobalNodeId },

    /// Catch type annotations are restricted to `any` and `unknown`.
    #[error(
        code = "EA246",
        message = "catch type annotations must be 'any' or 'unknown'"
    )]
    InvalidCatchAnnotationType { node: AnchoredGlobalNodeId },

    /// Type import targets must be string literals.
    #[error(
        code = "EA247",
        message = "type import target must be a string literal"
    )]
    InvalidTypeImportTarget { node: AnchoredGlobalNodeId },

    /// `new.target` is only valid in function or static block contexts.
    #[error(
        code = "EA248",
        message = "new.target is only valid in function or static block contexts"
    )]
    InvalidNewTarget { node: AnchoredGlobalNodeId },

    /// Object literal `__proto__` setters are not supported.
    #[error(
        code = "EA249",
        message = "object literal '__proto__' setters are not supported"
    )]
    UnsupportedObjectPrototypeSetter { node: AnchoredGlobalNodeId },

    /// New expressions cannot target optional chains.
    #[error(
        code = "EA250",
        message = "optional chaining cannot be used in new expressions"
    )]
    InvalidNewOptionalChain { node: AnchoredGlobalNodeId },

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

    /// Exceptions are disabled by configuration.
    #[error(code = "EA311", message = "exceptions are disabled")]
    ExceptionsDisabled { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 8xx: Restriction options
    // -------------------------------------------------------------------------
    /// The `any` type is disabled.
    #[error(code = "EA804", message = "any type is disabled")]
    AnyTypeDisabled { node: AnchoredGlobalNodeId },

    /// The `unknown` type is disabled.
    #[error(code = "EA805", message = "unknown type is disabled")]
    UnknownTypeDisabled { node: AnchoredGlobalNodeId },

    /// Imprecise primitive types are disabled.
    #[error(code = "EA806", message = "imprecise primitive type is disabled")]
    ImprecisePrimitiveDisabled { node: AnchoredGlobalNodeId },

    /// Unsafe type assertions are disabled.
    #[error(code = "EA807", message = "unsafe type assertions are disabled")]
    UnsafeTypeAssertionDisabled { node: AnchoredGlobalNodeId },

    /// Must assertions are disabled.
    #[error(code = "EA850", message = "must assertions are disabled")]
    MustAssertionDisabled { node: AnchoredGlobalNodeId },

    /// Definite assignment assertions are disabled.
    #[error(
        code = "EA851",
        message = "definite assignment assertions are disabled"
    )]
    DefiniteAssignmentAssertionDisabled { node: AnchoredGlobalNodeId },

    /// Custom type guards are disabled.
    #[error(code = "EA852", message = "custom type guards are disabled")]
    CustomTypeGuardDisabled { node: AnchoredGlobalNodeId },

    /// Untrusted declaration files are disabled.
    #[error(code = "EA853", message = "untrusted declaration files are disabled")]
    UntrustedDeclarationDisabled { node: AnchoredGlobalNodeId },

    /// Unsound variance rules are disabled.
    #[error(code = "EA854", message = "unsound variance is disabled")]
    UnsoundVarianceDisabled { node: AnchoredGlobalNodeId },

    /// Unsound narrowing rules are disabled.
    #[error(code = "EA855", message = "unsound narrowing is disabled")]
    UnsoundNarrowingDisabled { node: AnchoredGlobalNodeId },

    /// Dynamic imports are disabled.
    #[error(code = "EA808", message = "dynamic imports are disabled")]
    DynamicImportDisabled { node: AnchoredGlobalNodeId },

    /// Dynamic evaluation is disabled.
    #[error(code = "EA809", message = "dynamic evaluation is disabled")]
    DynamicEvaluationDisabled { node: AnchoredGlobalNodeId },

    /// Proxy usage is disabled.
    #[error(code = "EA810", message = "proxy usage is disabled")]
    ProxyDisabled { node: AnchoredGlobalNodeId },

    /// Dynamic shape mutation is disabled.
    #[error(code = "EA811", message = "dynamic shape mutation is disabled")]
    DynamicShapesDisabled { node: AnchoredGlobalNodeId },

    /// Computed property access is disabled.
    #[error(code = "EA812", message = "computed property access is disabled")]
    ComputedPropertyAccessDisabled { node: AnchoredGlobalNodeId },

    /// Referential equality is disabled.
    #[error(code = "EA813", message = "referential equality is disabled")]
    ReferentialEqualityDisabled { node: AnchoredGlobalNodeId },

    /// globalThis access is disabled.
    #[error(code = "EA814", message = "globalThis access is disabled")]
    GlobalThisDisabled { node: AnchoredGlobalNodeId },

    /// Implicit dynamic dispatch is disabled.
    #[error(code = "EA815", message = "implicit dynamic dispatch is disabled")]
    ImplicitDynamicDispatchDisabled { node: AnchoredGlobalNodeId },

    /// Implicit managed types are disabled.
    #[error(code = "EA816", message = "implicit managed types are disabled")]
    ImplicitManagedTypeDisabled { node: AnchoredGlobalNodeId },

    /// Implicit managed values are disabled.
    #[error(code = "EA817", message = "implicit managed values are disabled")]
    ImplicitManagedValueDisabled { node: AnchoredGlobalNodeId },

    /// Managed memory is disabled.
    #[error(code = "EA818", message = "managed memory is disabled")]
    ManagedMemoryDisabled { node: AnchoredGlobalNodeId },

    /// Runtime features are disabled.
    #[error(code = "EA819", message = "runtime features are disabled")]
    RuntimeDisabled { node: AnchoredGlobalNodeId },

    /// Implicit collection conversions are disabled.
    #[error(
        code = "EA820",
        message = "implicit collection conversions are disabled"
    )]
    ImplicitCollectionConversion { node: AnchoredGlobalNodeId },

    /// Strict mode forbids delete of unqualified bindings.
    #[error(
        code = "EA821",
        message = "delete target must be a property in strict mode"
    )]
    InvalidStrictDelete { node: AnchoredGlobalNodeId },

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

    /// Unreachable code.
    #[error(code = "EA312", message = "unreachable code")]
    UnreachableCode { node: AnchoredGlobalNodeId },

    /// Use of uninitialized variable in a read position.
    #[error(code = "EA306", message = "uninitialized variable")]
    UninitializedVariable { node: AnchoredGlobalNodeId },

    /// Incomplete try expression.
    #[error(code = "EA307", message = "try requires a catch or finally")]
    IncompleteTry { node: AnchoredGlobalNodeId },

    /// Try branch does not match the TryBranch shape.
    #[error(code = "EA320", message = "Try.branch must return TryBranch")]
    InvalidTryBranch { node: AnchoredGlobalNodeId },

    /// Try unwrap requires a Try return type.
    #[error(code = "EA321", message = "try unwrap requires a Try return type")]
    MissingTryReturnType { node: AnchoredGlobalNodeId },

    /// Invalid for of binding.
    #[error(
        code = "EA322",
        message = "invalid for of binding: `async` is reserved in this context"
    )]
    InvalidForOfBinding { node: AnchoredGlobalNodeId },

    /// Catch parameters in must be binding identifiers or binding patterns.
    #[error(
        code = "EA323",
        message = "invalid catch binding: expected identifier or binding pattern"
    )]
    InvalidCatchBinding { node: AnchoredGlobalNodeId },

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

    /// Named fields are not allowed in array or tuple patterns.
    #[error(
        code = "EA403",
        message = "named fields are not allowed in array or tuple patterns"
    )]
    InvalidPatternNamedField { node: AnchoredGlobalNodeId },

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
        abstraction: CallableAbstraction,
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
        abstraction: CallableAbstraction,
    },

    /// Missing override modifier for an overriding member.
    #[error(code = "EA509", message = "missing override modifier")]
    MissingOverride { node: AnchoredGlobalNodeId },

    /// Override modifier used without a matching base member.
    #[error(code = "EA510", message = "override does not match a base member")]
    InvalidOverride { node: AnchoredGlobalNodeId },

    /// Instance field is not initialized in every constructor.
    #[error(code = "EA511", message = "property is not definitely assigned")]
    UninitializedProperty { node: AnchoredGlobalNodeId },

    /// Optional parameters cannot use binding patterns.
    #[error(
        code = "EA512",
        message = "optional parameters cannot use binding patterns"
    )]
    InvalidOptionalPatternParameter { node: AnchoredGlobalNodeId },

    /// Rest parameters cannot be optional.
    #[error(code = "EA513", message = "optional rest parameters are not allowed")]
    InvalidOptionalRestParameter { node: AnchoredGlobalNodeId },

    /// Object members cannot declare duplicate field names.
    #[error(code = "EA514", message = "duplicate field '{field}'")]
    DuplicateField {
        node: AnchoredGlobalNodeId,
        field: StaticKey,
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

    /// Decorators cannot use static arguments.
    #[error(code = "EA701", message = "decorator static arguments are not allowed")]
    InvalidDecoratorStaticArguments { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 8xx: Options
    // -------------------------------------------------------------------------
    /// Unused local binding.
    #[error(code = "EA800", message = "unused local '{name}'")]
    UnusedLocal {
        node: AnchoredGlobalNodeId,
        name: StringId,
    },

    /// Unused parameter binding.
    #[error(code = "EA801", message = "unused parameter '{name}'")]
    UnusedParameter {
        node: AnchoredGlobalNodeId,
        name: StringId,
    },

    /// Unused label.
    #[error(code = "EA802", message = "unused label '{name}'")]
    UnusedLabel {
        node: AnchoredGlobalNodeId,
        name: StringId,
    },

    /// Switch case falls through to the next case.
    #[error(code = "EA803", message = "switch case falls through")]
    SwitchFallthrough { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[error(code = "EA900", message = "unsupported construct")]
    UnsupportedConstruct { node: AnchoredGlobalNodeId },

    /// TypeScript modules are disabled by configuration.
    #[error(
        code = "EA901",
        message = "typescript modules are disabled by configuration"
    )]
    TypeScriptDisabled { module: ModuleId },

    /// JavaScript modules are disabled by configuration.
    #[error(
        code = "EA902",
        message = "javascript modules are disabled by configuration"
    )]
    JavaScriptDisabled { module: ModuleId },

    /// Internal analyze error.
    #[error(code = "EA903", message = "internal error: {message}")]
    Internal { message: String },

    /// A required resolve product failed while analyzing.
    #[error(code = "EA904", message = "required resolve product failed: {message}")]
    FailedResolve {
        /// The underlying resolve error.
        error: Box<ResolveError>,
        /// Describe the resolve failure.
        message: String,
    },
}

impl AnalyzeError {
    /// Return true when this diagnostic is a cascading semantic consequence.
    pub fn is_cascading_semantic_diagnostic(&self) -> bool {
        matches!(
            self,
            Self::UnassignableType { .. }
                | Self::UnsatisfiedType { .. }
                | Self::ExcessProperty { .. }
                | Self::NonCallable { .. }
                | Self::MissingMember { .. }
                | Self::NoOverload { .. }
        )
    }
}

impl From<ResolveError> for AnalyzeError {
    fn from(error: ResolveError) -> Self {
        match error {
            ResolveError::Yield { requirement } => Self::Yield { requirement },
            ResolveError::UnsatisfiedRequirement { requirement } => {
                Self::UnsatisfiedRequirement { requirement }
            }
            ResolveError::Skipped => Self::Skipped,
            error => Self::FailedResolve {
                message: error.to_string(),
                error: Box::new(error),
            },
        }
    }
}
