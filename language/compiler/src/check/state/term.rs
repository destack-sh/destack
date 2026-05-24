use destack_dir as dir;
use smallvec::{SmallVec, smallvec};

use super::VariableId;

/// Term used to define a type variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeTerm {
    /// Concrete type.
    ///
    /// ```ts
    /// int32
    /// ```
    Literal(dir::Type),
    /// Source intrinsic marker.
    ///
    /// ```ts
    /// type T = intrinsic;
    /// ```
    ///
    /// The marker is valid only as the declaration body of compiler recognized language items.
    Intrinsic,
    /// Source `const` assertion marker.
    ///
    /// ```ts
    /// value as const
    /// ```
    ///
    /// The marker is valid only as the target of a const assertion expression.
    ConstAssertion,
    /// Type variable.
    ///
    /// ```ts
    /// let y = x;
    /// ```
    ///
    /// The type of `y` can alias the type variable for `x`.
    Variable(VariableId),
    /// Canonical memory form over a value type.
    ///
    /// ```ts
    /// &T
    /// ```
    ///
    /// The syntax becomes a borrowed form over payload `T`.
    Form {
        /// The form constructor.
        form: FormTerm,
        /// The carried payload type.
        payload: VariableId,
    },
    /// Named type declaration reference.
    ///
    /// ```ts
    /// Map<K, V>
    /// ```
    Reference {
        /// The source type expression that wrote this reference.
        source: Option<dir::GlobalNodeIdAny>,
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The applied static arguments.
        arguments: Vec<ArgumentTerm>,
    },
    /// Homogeneous array type.
    Array {
        /// The element type.
        element: VariableId,
    },
    /// Type member projection.
    ///
    /// ```ts
    /// T.Item
    /// ```
    Member {
        /// The source member expression when this term came from runtime syntax.
        source: Option<dir::GlobalNodeIdAny>,
        /// The owner type.
        owner: VariableId,
        /// The selected member key.
        key: dir::StaticKey,
        /// The applied static arguments.
        arguments: Vec<ArgumentTerm>,
    },
    /// Fixed-length array type.
    FixedArray {
        /// The repeated element type.
        element: VariableId,
        /// The static array length.
        length: VariableId,
        /// Whether the array is readonly.
        is_readonly: bool,
    },
    /// Runtime-length homogeneous view type.
    Slice {
        /// The element type.
        element: VariableId,
        /// Whether the slice is readonly.
        is_readonly: bool,
    },
    /// Tuple type.
    Tuple {
        /// The tuple source form.
        form: dir::TupleForm,
        /// The tuple elements.
        elements: Vec<TupleElementTerm>,
        /// Whether the tuple is readonly.
        is_readonly: bool,
    },
    /// Structural object shape type.
    Shape {
        /// The shape members.
        members: Vec<ShapeMemberTerm>,
    },
    /// Function type.
    Function(FunctionTerm),
    /// Compact scalar interval type.
    Range {
        /// The inclusive lower bound.
        start: Option<dir::ScalarLiteral>,
        /// The upper bound.
        end: Option<dir::ScalarLiteral>,
        /// Whether the upper bound is included.
        is_inclusive: bool,
    },
    /// Union type.
    Union {
        /// The union elements.
        elements: Vec<VariableId>,
    },
    /// Intersection type.
    Intersection {
        /// The intersection elements.
        elements: Vec<VariableId>,
    },
    /// Type-level operation.
    ///
    /// ```ts
    /// keyof T
    /// ```
    Operation(TypeOperationTerm),
    /// Runtime call expression.
    ///
    /// ```ts
    /// fn(value)
    /// ```
    Call(CallTerm),
    /// Runtime construct expression.
    ///
    /// ```ts
    /// new User(value)
    /// ```
    Construct(ConstructTerm),
    /// Runtime operator expression.
    ///
    /// ```ts
    /// -value
    /// left + right
    /// ```
    Operator(OperatorTerm),
    /// Runtime index access.
    ///
    /// ```ts
    /// value[key]
    /// ```
    Index(IndexTerm),
    /// Runtime key membership check.
    ///
    /// ```ts
    /// "name" in value
    /// ```
    KeyMembership(KeyMembershipTerm),
    /// Runtime nominal instance check.
    ///
    /// ```ts
    /// value instanceof Error
    /// ```
    InstanceCheck(InstanceCheckTerm),
    /// Runtime identity equality check.
    ///
    /// ```ts
    /// left === right
    /// ```
    Identity(IdentityTerm),
    /// Runtime await expression.
    ///
    /// ```ts
    /// await value
    /// ```
    Await(AwaitTerm),
    /// Runtime try expression.
    ///
    /// ```ts
    /// value?
    /// ```
    Try(TryTerm),
    /// Runtime template string expression.
    ///
    /// ```ts
    /// `hello ${name}`
    /// ```
    Template(TemplateTerm),
    /// Runtime tagged template expression.
    ///
    /// ```ts
    /// sql`select ${id}`
    /// ```
    TaggedTemplate(TaggedTemplateTerm),
    /// Type predicate.
    Predicate {
        /// Whether this is an assertion predicate.
        asserts: bool,
        /// The predicate subject.
        subject: dir::PredicateSubject,
        /// The predicate target type.
        target: Option<VariableId>,
    },
}

/// Term used to define a static variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticTerm {
    /// Concrete static term.
    ///
    /// ```ts
    /// "mutable"
    /// ```
    Literal(dir::StaticTerm),
    /// Static variable alias.
    ///
    /// ```ts
    /// import { L } from "./lifetimes";
    /// ```
    Variable(VariableId),
    /// Source expression evaluated as a static term.
    ///
    /// ```ts
    /// N + 1
    /// ```
    Expression(dir::GlobalNodeId<dir::Expression>),
    /// Static member projection.
    ///
    /// ```ts
    /// Register.Width
    /// ```
    Member {
        /// The source member expression when available.
        source: Option<dir::GlobalNodeIdAny>,
        /// The owner type.
        owner: VariableId,
        /// The selected member key.
        key: dir::StaticKey,
        /// The applied static arguments.
        arguments: Vec<ArgumentTerm>,
    },
    /// Lifetime union produced by type-position `|`.
    ///
    /// ```ts
    /// L | R
    /// ```
    LifetimeJoin {
        /// The lifetime values being joined.
        elements: Vec<VariableId>,
    },
    /// Compiler intrinsic returning a static value.
    ///
    /// ```ts
    /// LifetimeOf<T>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
}

/// Check-local memory form term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum FormTerm {
    /// Managed value form.
    Managed,
    /// Owned value form.
    Owned,
    /// Borrowed value form.
    Borrowed {
        /// The solved lifetime value.
        lifetime: VariableId,
        /// The solved access value.
        access: VariableId,
    },
    /// Raw pointer form.
    Raw,
    /// Placed value form.
    Placed {
        /// The solved place value.
        place: VariableId,
    },
    /// Readonly view form.
    Readonly,
}

/// Argument supplied to a generic use.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ArgumentTerm {
    /// Type argument.
    Type(VariableId),
    /// Static argument.
    Static(VariableId),
    /// Spread type argument.
    SpreadType(VariableId),
    /// Static spread argument.
    SpreadStatic(VariableId),
}

/// Function type term.
///
/// ```ts
/// (value: string) => int32
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionTerm {
    /// The function asynchrony.
    pub(in crate::check) asynchrony: dir::Asynchrony,
    /// The generic parameter types.
    pub(in crate::check) generic_parameters: Vec<VariableId>,
    /// The optional `this` parameter type.
    pub(in crate::check) this_parameter: Option<VariableId>,
    /// The parameter types.
    pub(in crate::check) parameters: Vec<VariableId>,
    /// The optional return type.
    pub(in crate::check) return_type: Option<VariableId>,
    /// Whether this is a generator function.
    pub(in crate::check) is_generator: bool,
}

/// Tuple element term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TupleElementTerm {
    /// The optional label for the element.
    pub(in crate::check) label: Option<dir::StringId>,
    /// The element type.
    pub(in crate::check) ty: VariableId,
    /// Whether the element is optional.
    pub(in crate::check) is_optional: bool,
    /// Whether the element is readonly.
    pub(in crate::check) is_readonly: bool,
    /// Whether the element is a rest element.
    pub(in crate::check) is_rest: bool,
}

/// Mapped type parameter term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MappedParameterTerm {
    /// The parameter name.
    pub(in crate::check) name: dir::StringId,
    /// The parameter symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source type iterated by `in`.
    pub(in crate::check) constraint: VariableId,
    /// The optional key remap.
    pub(in crate::check) key_remap: Option<VariableId>,
}

/// Shape member term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ShapeMemberTerm {
    /// Shape field.
    Field {
        /// The field key.
        key: dir::StaticKey,
        /// The field type.
        ty: VariableId,
        /// Whether the field is optional.
        is_optional: bool,
        /// Whether the field is readonly.
        is_readonly: bool,
    },
    /// Call signature.
    CallSignature {
        /// The signature type.
        ty: VariableId,
    },
    /// Construct signature.
    ConstructSignature {
        /// The signature type.
        ty: VariableId,
    },
    /// Index signature.
    IndexSignature {
        /// The parameter name.
        name: dir::StringId,
        /// The key type.
        key_type: VariableId,
        /// The value type.
        value_type: VariableId,
        /// Whether the index signature is optional.
        is_optional: bool,
        /// Whether the index signature is readonly.
        is_readonly: bool,
    },
}

/// Type-level operation term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeOperationTerm {
    /// Conditional type expression.
    ///
    /// ```ts
    /// T extends string ? A : B
    /// ```
    Conditional {
        /// The left operand.
        left: VariableId,
        /// The right operand.
        right: VariableId,
        /// The type selected when the condition holds.
        then_type: VariableId,
        /// The type selected when the condition does not hold.
        else_type: VariableId,
    },
    /// Indexed access type expression.
    ///
    /// ```ts
    /// T[K]
    /// ```
    Index {
        /// The indexed type.
        left: VariableId,
        /// The index type.
        index: VariableId,
    },
    /// Template literal type expression.
    ///
    /// ```ts
    /// `id:${T}`
    /// ```
    TemplateLiteral {
        /// The literal string segments.
        strings: Vec<dir::StringId>,
        /// The interpolated type spans.
        spans: Vec<VariableId>,
    },
    /// Type infer binding in a conditional type pattern.
    ///
    /// ```ts
    /// T extends Array<infer U> ? U : never
    /// ```
    Infer {
        /// The inferred binding name.
        name: Option<dir::StringId>,
        /// The optional inferred constraint.
        constraint: Option<VariableId>,
    },
    /// `keyof T`.
    KeyOf {
        /// The target type.
        target: VariableId,
    },
    /// Mapped type expression.
    ///
    /// ```ts
    /// { [K in keyof T]: T[K] }
    /// ```
    Mapped {
        /// The mapped parameter.
        parameter: MappedParameterTerm,
        /// The mapped modifiers.
        modifiers: dir::MappedTypeModifiers,
        /// The mapped value type.
        value: VariableId,
    },
    /// Best common type selected for expression literals.
    ///
    /// ```ts
    /// [left, right]
    /// ```
    BestCommon {
        /// The candidate element types.
        elements: Vec<VariableId>,
    },
    /// Literal widening for inferred mutable storage.
    ///
    /// ```ts
    /// let value = 1
    /// ```
    Widen {
        /// The inferred source type.
        source: VariableId,
    },
    /// Type exclusion expression.
    ///
    /// ```ts
    /// Exclude<T, null>
    /// ```
    Exclude {
        /// The source type.
        source: VariableId,
        /// The excluded type.
        target: VariableId,
    },
    /// Compiler intrinsic returning a type.
    ///
    /// ```ts
    /// WithLifetime<T, L>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
}

/// Symbol-backed callable candidate.
///
/// ```ts
/// value.toString()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CallCandidateTerm {
    /// The selected callable target.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The candidate callable type.
    pub(in crate::check) ty: VariableId,
}

/// Runtime call expression term.
///
/// ```ts
/// format(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallTerm {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The called expression type.
    pub(in crate::check) callee: VariableId,
    /// The member projection that produced this callee.
    pub(in crate::check) member: Option<MemberCallTerm>,
    /// The symbol-backed candidates visible at the call site.
    pub(in crate::check) candidates: Vec<CallCandidateTerm>,
    /// The explicit call generic arguments.
    pub(in crate::check) generic_arguments: Vec<ArgumentTerm>,
    /// The argument expression types.
    pub(in crate::check) arguments: Vec<VariableId>,
}

/// Runtime construct expression term.
///
/// ```ts
/// new User(name)
/// new Ctor()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructTerm {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The constructed expression type.
    pub(in crate::check) callee: VariableId,
    /// The explicit construct generic arguments.
    pub(in crate::check) generic_arguments: Vec<ArgumentTerm>,
    /// The argument expression types.
    pub(in crate::check) arguments: Vec<VariableId>,
}

/// Member projection used as a runtime call callee.
///
/// ```ts
/// value.toString()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberCallTerm {
    /// The receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: Vec<ArgumentTerm>,
    /// The protocol that must own the resolved method.
    pub(in crate::check) protocol: Option<MemberProtocol>,
}

/// Protocol contract required for a member resolution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberProtocol {
    /// The protocol language item.
    pub(in crate::check) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::check) arguments: Vec<ArgumentTerm>,
}

/// Runtime index access term.
///
/// ```ts
/// values[index]
/// tuple[0]
/// ```
///
/// The solver decides whether the access is a structural projection or an `Index` protocol call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IndexTerm {
    /// The source index expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The indexed receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The index expression type.
    pub(in crate::check) index: VariableId,
    /// The direct structural key when syntax makes it obvious.
    pub(in crate::check) key: Option<dir::StaticKey>,
}

/// Runtime key membership check term.
///
/// ```ts
/// key in value
/// ```
///
/// The solver resolves this as structural key membership or a protocol call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct KeyMembershipTerm {
    /// The source membership expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The key expression type.
    pub(in crate::check) key: VariableId,
    /// The receiver expression type.
    pub(in crate::check) receiver: VariableId,
    /// The direct key when syntax makes it statically obvious.
    pub(in crate::check) static_key: Option<dir::StaticKey>,
}

/// Runtime nominal instance check term.
///
/// ```ts
/// value instanceof Shape.Circle
/// ```
///
/// The solver checks a runtime tag-compatible nominal target and records narrowing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct InstanceCheckTerm {
    /// The source instance check expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked value type.
    pub(in crate::check) value: VariableId,
    /// The target constructor or nominal type value.
    pub(in crate::check) target: VariableId,
}

/// Runtime identity equality term.
///
/// ```ts
/// left !== right
/// ```
///
/// The solver accepts only identity-compatible operands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct IdentityTerm {
    /// The source identity expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) operator: dir::BinaryOperator,
    /// The left operand type.
    pub(in crate::check) left: VariableId,
    /// The right operand type.
    pub(in crate::check) right: VariableId,
}

/// Runtime await expression term.
///
/// ```ts
/// await task
/// ```
///
/// The solver unwraps the `Promise<T>` language item to `T`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct AwaitTerm {
    /// The source await expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The awaited expression type.
    pub(in crate::check) value: VariableId,
}

/// Runtime try operator term.
///
/// ```ts
/// result?
/// result!
/// ```
///
/// The solver projects `Try.Value`, checks the `Try` protocol, and validates propagation targets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TryTerm {
    /// The source try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tried expression type.
    pub(in crate::check) value: VariableId,
    /// The try operator behavior.
    pub(in crate::check) kind: TryTermKind,
}

/// Runtime template string term.
///
/// ```ts
/// `/${prefix}/${id}`
/// ```
///
/// The solver yields a precise literal string when all spans are static.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TemplateTerm {
    /// The source template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<VariableId>,
}

/// Runtime tagged template term.
///
/// ```ts
/// sql<User>`select ${id}`
/// ```
///
/// The solver handles this as a call with a structured template argument.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TaggedTemplateTerm {
    /// The source tagged template expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The tag expression type.
    pub(in crate::check) tag: VariableId,
    /// The explicit tag generic arguments.
    pub(in crate::check) generic_arguments: Vec<ArgumentTerm>,
    /// The literal string segments.
    pub(in crate::check) strings: Vec<dir::StringId>,
    /// The interpolated expression types.
    pub(in crate::check) spans: Vec<VariableId>,
}

/// Try operator behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TryTermKind {
    /// Propagate failure through the enclosing return type.
    Propagate,
    /// Trap failure and produce the successful value.
    Trap,
}

/// Runtime operator expression term.
///
/// ```ts
/// -value
/// left + right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct OperatorTerm {
    /// The source operator expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) kind: OperatorTermKind,
    /// The receiver operand type.
    pub(in crate::check) receiver: VariableId,
    /// The remaining operand type.
    pub(in crate::check) argument: Option<VariableId>,
}

/// Source operator represented by an operator term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum OperatorTermKind {
    /// Unary source operator.
    Unary(dir::UnaryOperator),
    /// Binary source operator.
    Binary(dir::BinaryOperator),
}

impl ArgumentTerm {
    /// Return the variable referenced by this argument.
    pub(in crate::check) fn variable(&self) -> VariableId {
        match self {
            Self::Type(variable)
            | Self::Static(variable)
            | Self::SpreadType(variable)
            | Self::SpreadStatic(variable) => *variable,
        }
    }
}

impl TypeTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Variable(variable) => variables.push(*variable),
            Self::Form { form, payload } => {
                variables.push(*payload);
                variables.extend(form.referenced_variables());
            }
            Self::Reference {
                source: _,
                symbol: _,
                arguments,
            } => {
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::Array { element } => variables.push(*element),
            Self::Member {
                source: _,
                owner,
                key: _,
                arguments,
            } => {
                variables.push(*owner);
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::FixedArray {
                element,
                length,
                is_readonly: _,
            } => {
                variables.push(*element);
                variables.push(*length);
            }
            Self::Slice {
                element,
                is_readonly: _,
            } => variables.push(*element),
            Self::Tuple {
                form: _,
                elements,
                is_readonly: _,
            } => {
                variables.extend(elements.iter().map(|element| element.ty));
            }
            Self::Shape { members } => {
                for member in members {
                    variables.extend(member.referenced_variables());
                }
            }
            Self::Function(function) => variables.extend(function.referenced_variables()),
            Self::Range {
                start: _,
                end: _,
                is_inclusive: _,
            } => {}
            Self::Union { elements } | Self::Intersection { elements } => {
                variables.extend(elements.iter().copied());
            }
            Self::Operation(operation) => variables.extend(operation.referenced_variables()),
            Self::Call(call) => variables.extend(call.referenced_variables()),
            Self::Construct(construct) => variables.extend(construct.referenced_variables()),
            Self::Operator(operator) => variables.extend(operator.referenced_variables()),
            Self::Index(index) => variables.extend(index.referenced_variables()),
            Self::KeyMembership(membership) => {
                variables.extend(membership.referenced_variables());
            }
            Self::InstanceCheck(instance) => variables.extend(instance.referenced_variables()),
            Self::Identity(identity) => variables.extend(identity.referenced_variables()),
            Self::Await(awaited) => variables.push(awaited.value),
            Self::Try(tried) => variables.extend(tried.referenced_variables()),
            Self::Template(template) => variables.extend(template.referenced_variables()),
            Self::TaggedTemplate(template) => variables.extend(template.referenced_variables()),
            Self::Predicate {
                asserts: _,
                subject: _,
                target,
            } => variables.extend(target.iter().copied()),
            Self::Literal(_) | Self::Intrinsic | Self::ConstAssertion => {}
        }

        variables
    }
}

impl FunctionTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.extend(self.generic_parameters.iter().copied());
        variables.extend(self.this_parameter);
        variables.extend(self.parameters.iter().copied());
        variables.extend(self.return_type.iter().copied());

        variables
    }
}

impl MappedParameterTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.constraint);
        variables.extend(self.key_remap);

        variables
    }
}

impl ShapeMemberTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Field {
                key: _,
                ty,
                is_optional: _,
                is_readonly: _,
            }
            | Self::CallSignature { ty }
            | Self::ConstructSignature { ty } => variables.push(*ty),
            Self::IndexSignature {
                name: _,
                key_type,
                value_type,
                is_optional: _,
                is_readonly: _,
            } => {
                variables.push(*key_type);
                variables.push(*value_type);
            }
        }

        variables
    }
}

impl TypeOperationTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => smallvec![*left, *right, *then_type, *else_type],
            Self::Index { left, index } => smallvec![*left, *index],
            Self::TemplateLiteral { strings: _, spans } => spans.iter().copied().collect(),
            Self::Infer {
                name: _,
                constraint,
            } => constraint.iter().copied().collect(),
            Self::KeyOf { target } => smallvec![*target],
            Self::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                let mut variables = parameter.referenced_variables();

                variables.push(*value);

                variables
            }
            Self::BestCommon { elements } => elements.iter().copied().collect(),
            Self::Widen { source } => smallvec![*source],
            Self::Exclude { source, target } => smallvec![*source, *target],
            Self::Intrinsic { item: _, arguments } => {
                arguments.iter().map(ArgumentTerm::variable).collect()
            }
        }
    }
}

impl CallTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        if self.member.is_none() && self.candidates.is_empty() {
            variables.push(self.callee);
        }
        if !self.candidates.is_empty() {
            variables.extend(self.candidates.iter().map(|candidate| candidate.ty));
        }
        if let Some(member) = &self.member {
            variables.push(member.receiver);
            variables.extend(member.arguments.iter().map(ArgumentTerm::variable));
        }
        variables.extend(self.generic_arguments.iter().map(ArgumentTerm::variable));
        variables.extend(self.arguments.iter().copied());

        variables
    }
}

impl ConstructTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.callee);
        variables.extend(self.generic_arguments.iter().map(ArgumentTerm::variable));
        variables.extend(self.arguments.iter().copied());

        variables
    }
}

impl IndexTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.receiver);
        variables.push(self.index);

        variables
    }
}

impl KeyMembershipTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec![self.key, self.receiver]
    }
}

impl InstanceCheckTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec![self.value, self.target]
    }
}

impl IdentityTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec![self.left, self.right]
    }
}

impl OperatorTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();
        variables.push(self.receiver);
        variables.extend(self.argument);
        variables
    }
}

impl TemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        self.spans.iter().copied().collect()
    }
}

impl TaggedTemplateTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();
        variables.push(self.tag);
        variables.extend(self.generic_arguments.iter().map(ArgumentTerm::variable));
        variables.extend(self.spans.iter().copied());
        variables
    }
}

impl TryTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();
        variables.push(self.value);
        variables
    }
}

impl StaticTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Variable(variable) => variables.push(*variable),
            Self::Member {
                source: _,
                owner,
                key: _,
                arguments,
            } => {
                variables.push(*owner);
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::LifetimeJoin { elements } => variables.extend(elements.iter().copied()),
            Self::Intrinsic { item: _, arguments } => {
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::Literal(_) | Self::Expression(_) => {}
        }

        variables
    }
}

impl FormTerm {
    /// Return whether two forms share the same constructor.
    pub(in crate::check) fn same_constructor(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Owned, Self::Owned)
                | (Self::Managed, Self::Managed)
                | (
                    Self::Borrowed {
                        lifetime: _,
                        access: _,
                    },
                    Self::Borrowed {
                        lifetime: _,
                        access: _,
                    },
                )
                | (Self::Raw, Self::Raw)
                | (Self::Placed { place: _ }, Self::Placed { place: _ })
                | (Self::Readonly, Self::Readonly)
        )
    }

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Borrowed { lifetime, access } => {
                variables.push(*lifetime);
                variables.push(*access);
            }
            Self::Placed { place } => variables.push(*place),
            Self::Managed | Self::Owned | Self::Raw | Self::Readonly => {}
        }

        variables
    }
}
