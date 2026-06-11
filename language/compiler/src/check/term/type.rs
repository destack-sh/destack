use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, AwaitTerm, CallCallee, CallTerm, CheckEvent, CheckState, Condition, ConstructTerm,
    Definition, Dependency, FormTerm, FunctionTerm, GenericArgument, GenericParameterBinding,
    GenericParameterId, IdentityTerm, ImportMetaTerm, IndexSetTerm, IndexTerm, InstanceCheckTerm,
    KeyMembershipTerm, MemberCallTerm, MemberLookup, MemberProjectionOrigin, MemberProtocol,
    MemberReceiver, MemberTerm, OperatorTerm, Origin, RangeTerm, ReceiverTerm, ShapeMember,
    ShapeTerm, StaticOperand, StaticRelation, StaticTerm, SubstitutionSet, SuperTerm,
    TaggedTemplateTerm, TemplateTerm, TermId, TreeTerm, TryFailureTerm, TryTerm, TupleElement,
    TypeOperand, TypeOperationTerm, TypeRelation, TypeValueTerm, VariableId, YieldTerm,
};
use crate::{CompilerError, CompilerResult};

/// Literal type value with no nested table references.
///
/// Examples:
/// ```ds
/// int32
/// "ready"
/// null
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum TypeLiteralTerm {
    /// Error type that could not be resolved.
    ///
    /// Examples:
    /// ```ds
    /// MissingType
    /// ```
    Error,
    /// Never type.
    ///
    /// Examples:
    /// ```ds
    /// never
    /// ```
    Never,
    /// TypeScript `any` compatibility marker.
    ///
    /// Examples:
    /// ```ds
    /// any
    /// ```
    Any,
    /// Unknown type.
    ///
    /// Examples:
    /// ```ds
    /// unknown
    /// ```
    Unknown,
    /// Void type.
    ///
    /// Examples:
    /// ```ds
    /// void
    /// ```
    Void,
    /// Null type and value.
    ///
    /// Examples:
    /// ```ds
    /// null
    /// ```
    Null,
    /// Undefined type and value.
    ///
    /// Examples:
    /// ```ds
    /// undefined
    /// ```
    Undefined,
    /// TypeScript object constraint.
    ///
    /// Examples:
    /// ```ds
    /// object
    /// ```
    Object,
    /// Primitive type.
    ///
    /// Examples:
    /// ```ds
    /// int32
    /// string
    /// ```
    Primitive(dir::PrimitiveType),
    /// Scalar literal type.
    ///
    /// Examples:
    /// ```ds
    /// "ready"
    /// 1
    /// ```
    Scalar(dir::ScalarLiteral),
}

// assert that TypeLiteralTerm <= 64B
const _: () = assert!(std::mem::size_of::<TypeLiteralTerm>() <= 64);

impl TypeLiteralTerm {
    /// Return whether this literal is null.
    pub(in crate::check) fn is_null(&self) -> bool {
        matches!(self, Self::Null | Self::Scalar(dir::ScalarLiteral::Null))
    }

    /// Return whether this literal is undefined.
    pub(in crate::check) fn is_undefined(&self) -> bool {
        matches!(
            self,
            Self::Undefined | Self::Scalar(dir::ScalarLiteral::Undefined)
        )
    }

    /// Return the builtin boolean type.
    pub(in crate::check) fn boolean() -> Self {
        Self::Primitive(dir::PrimitiveType::Boolean)
    }

    /// Return whether this literal is nullish.
    pub(in crate::check) fn is_nullish(&self) -> bool {
        self.is_null() || self.is_undefined()
    }

    /// Return whether both literals are the same nullish literal.
    pub(in crate::check) fn is_same_nullish_literal(&self, other: &Self) -> bool {
        (self.is_null() && other.is_null()) || (self.is_undefined() && other.is_undefined())
    }

    /// Return whether both literals support builtin value equality.
    pub(in crate::check) fn has_builtin_value_equality_with(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (
                Self::Primitive(dir::PrimitiveType::Boolean)
                    | Self::Scalar(dir::ScalarLiteral::Boolean(_)),
                Self::Primitive(dir::PrimitiveType::Boolean)
                    | Self::Scalar(dir::ScalarLiteral::Boolean(_)),
            ) | (
                Self::Primitive(dir::PrimitiveType::Character)
                    | Self::Scalar(dir::ScalarLiteral::Character(_)),
                Self::Primitive(dir::PrimitiveType::Character)
                    | Self::Scalar(dir::ScalarLiteral::Character(_)),
            ) | (
                Self::Scalar(dir::ScalarLiteral::String(_)),
                Self::Scalar(dir::ScalarLiteral::String(_)),
            )
        )
    }

    /// Return whether this literal supports strict identity.
    pub(in crate::check) fn has_strict_identity(self) -> bool {
        if self.is_nullish() {
            return true;
        }

        matches!(
            self,
            Self::Primitive(dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol)
        )
    }

    /// Return the builtin number type.
    pub(in crate::check) fn number() -> Self {
        Self::Primitive(dir::PrimitiveType::Float(dir::FloatType::Float64))
    }

    /// Return the builtin integer family type.
    pub(in crate::check) fn integer() -> Self {
        Self::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Integer {
            is_signed: true,
        }))
    }

    /// Return the builtin bigint type.
    pub(in crate::check) fn bigint() -> Self {
        Self::Primitive(dir::PrimitiveType::Bigint)
    }

    /// Convert one source type literal into a solver literal.
    pub(in crate::check) fn from_literal(value: &dir::TypeLiteral) -> Self {
        match value {
            dir::TypeLiteral::Never => Self::Never,
            dir::TypeLiteral::Any => Self::Any,
            dir::TypeLiteral::Undefined => Self::Undefined,
            dir::TypeLiteral::Unknown => Self::Unknown,
            dir::TypeLiteral::Object => Self::Object,
            dir::TypeLiteral::Void => Self::Void,
            dir::TypeLiteral::Null => Self::Null,
            dir::TypeLiteral::Boolean => Self::boolean(),
            dir::TypeLiteral::Character => Self::Primitive(dir::PrimitiveType::Character),
            dir::TypeLiteral::String => Self::Primitive(dir::PrimitiveType::String),
            dir::TypeLiteral::Bigint => Self::bigint(),
            dir::TypeLiteral::Number => Self::number(),
            dir::TypeLiteral::Integer(integer) => {
                Self::Primitive(dir::PrimitiveType::Integer(*integer))
            }
            dir::TypeLiteral::Float(float) => Self::Primitive(dir::PrimitiveType::Float(*float)),
            dir::TypeLiteral::Symbol => Self::Primitive(dir::PrimitiveType::Symbol),
            dir::TypeLiteral::UniqueSymbol => Self::Primitive(dir::PrimitiveType::UniqueSymbol),
        }
    }

    /// Convert one literal DIR type into a solver literal.
    pub(in crate::check) fn from_type(ty: &dir::Type) -> Option<Self> {
        let literal = match ty {
            dir::Type::Error => Self::Error,
            dir::Type::Never => Self::Never,
            dir::Type::Any => Self::Any,
            dir::Type::Unknown => Self::Unknown,
            dir::Type::Void => Self::Void,
            dir::Type::Null => Self::Null,
            dir::Type::Undefined => Self::Undefined,
            dir::Type::Object => Self::Object,
            dir::Type::Primitive(primitive) => Self::Primitive(*primitive),
            dir::Type::Literal(literal) => Self::Scalar(*literal),
            _ => return None,
        };

        Some(literal)
    }

    /// Convert this literal into a committed type.
    pub(in crate::check) fn to_type(&self) -> dir::Type {
        match self {
            Self::Error => dir::Type::Error,
            Self::Never => dir::Type::Never,
            Self::Any => dir::Type::Any,
            Self::Unknown => dir::Type::Unknown,
            Self::Void => dir::Type::Void,
            Self::Null => dir::Type::Null,
            Self::Undefined => dir::Type::Undefined,
            Self::Object => dir::Type::Object,
            Self::Primitive(primitive) => dir::Type::Primitive(*primitive),
            Self::Scalar(literal) => dir::Type::Literal(*literal),
        }
    }
}

/// Term used to define a type variable.
///
/// Examples:
/// ```ds
/// const value = fn(input)
/// type Item = Box<T>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) enum TypeTerm {
    /// Committed type.
    ///
    /// ```ds
    /// import { Value } from "./dependency"
    /// ```
    Type(dir::GlobalTypeId),
    /// Literal concrete type.
    ///
    /// ```ds
    /// int32
    /// ```
    Literal(TypeLiteralTerm),
    /// Compiler intrinsic type body.
    ///
    /// ```ds
    /// intrinsic
    /// ```
    Intrinsic,
    /// Generic parameter reference.
    ///
    /// ```ds
    /// T
    /// ```
    Parameter(GenericParameterId),
    /// Type declaration reference.
    ///
    /// ```ds
    /// Map<K, V>
    /// ```
    Reference {
        /// The work origin that introduced this reference.
        origin: Origin,
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The applied static arguments.
        arguments: SmallVec<[GenericArgument; 2]>,
    },
    /// This type.
    ///
    /// ```ds
    /// this
    /// ```
    This,
    /// Type member projection.
    ///
    /// ```ds
    /// T.Item
    /// ```
    Member(TermId<MemberTerm>),
    /// Canonical memory form over a value type.
    ///
    /// ```ds
    /// &T
    /// ```
    ///
    /// The syntax becomes a borrowed form over payload `T`.
    Form {
        /// The form constructor.
        form: TermId<FormTerm>,
        /// The carried payload type.
        payload: TypeOperand,
    },
    /// Explicit runtime Dynamic type.
    ///
    /// ```ds
    /// Dynamic<T>
    /// ```
    Dynamic {
        /// The Dynamic constraint type.
        constraint: TypeOperand,
    },
    /// Type-level operation.
    ///
    /// ```ds
    /// keyof T
    /// ```
    Operation(TermId<TypeOperationTerm>),
    /// Homogeneous array type.
    ///
    /// ```ds
    /// string[]
    /// Array<string>
    /// ```
    Array {
        /// The element type.
        element: TypeOperand,
    },
    /// Fixed-length array type.
    ///
    /// ```ds
    /// [int32; 4]
    /// ```
    FixedArray {
        /// The repeated element type.
        element: TypeOperand,
        /// The static array length.
        length: StaticOperand,
    },
    /// Compact scalar interval type.
    ///
    /// ```ds
    /// 0..10
    /// 0..=10
    /// ```
    Range {
        /// The inclusive lower bound.
        start: Option<dir::ScalarLiteral>,
        /// The upper bound.
        end: Option<dir::ScalarLiteral>,
        /// Whether the upper bound is included.
        is_inclusive: bool,
    },
    /// Runtime-length homogeneous view type.
    ///
    /// ```ds
    /// &[int32]
    /// ```
    Slice {
        /// The element type.
        element: TypeOperand,
    },
    /// Tuple type.
    ///
    /// ```ds
    /// [string, int32]
    /// ```
    Tuple {
        /// The tuple source form.
        form: dir::TupleForm,
        /// The tuple elements.
        elements: Vec<TupleElement>,
    },
    /// Structural object shape type.
    ///
    /// ```ds
    /// { name: string, age?: int32 }
    /// ```
    Shape(TermId<ShapeTerm>),
    /// Function type.
    ///
    /// ```ds
    /// (value: string) => int32
    /// ```
    Function(TermId<FunctionTerm>),
    /// Closure type with its captured environment.
    ///
    /// ```ds
    /// () => value
    /// ```
    Closure {
        /// The function contract type.
        function: TypeOperand,
        /// The captured environment type.
        environment: TypeOperand,
    },
    /// Union type.
    ///
    /// ```ds
    /// string | null
    /// ```
    Union {
        /// The union elements.
        elements: Vec<TypeOperand>,
    },
    /// Intersection type.
    ///
    /// ```ds
    /// Named & Timestamped
    /// ```
    Intersection {
        /// The intersection elements.
        elements: Vec<TypeOperand>,
    },
    /// Static value projected into type position.
    ///
    /// ```ds
    /// type Tagged<comptime Tag: string> = { tag: Tag };
    /// ```
    StaticValue {
        /// The static value operand.
        value: StaticOperand,
    },
    /// Runtime call expression.
    ///
    /// ```ds
    /// fn(value)
    /// ```
    Call(TermId<CallTerm>),
    /// Runtime construct expression.
    ///
    /// ```ds
    /// new User(value)
    /// ```
    Construct(TermId<ConstructTerm>),
    /// Runtime range value expression.
    ///
    /// ```ds
    /// start..end
    /// ```
    RangeValue(TermId<RangeTerm>),
    /// Runtime tree expression.
    ///
    /// ```dsx
    /// <Tag />
    /// ```
    Tree(TermId<TreeTerm>),
    /// Runtime reflected type value.
    ///
    /// ```ds
    /// type T
    /// ```
    TypeValue(TermId<TypeValueTerm>),
    /// Runtime import metadata value.
    ///
    /// ```ds
    /// import.meta
    /// ```
    ImportMeta(TermId<ImportMetaTerm>),
    /// Runtime contextual receiver.
    ///
    /// ```ds
    /// this
    /// ```
    Receiver(TermId<ReceiverTerm>),
    /// Runtime super receiver context.
    ///
    /// ```ds
    /// super
    /// ```
    Super(TermId<SuperTerm>),
    /// Runtime operator expression.
    ///
    /// ```ds
    /// -value
    /// left + right
    /// ```
    Operator(TermId<OperatorTerm>),
    /// Runtime index access.
    ///
    /// ```ds
    /// value[key]
    /// ```
    Index(TermId<IndexTerm>),
    /// Runtime index set.
    ///
    /// ```ds
    /// value[key] = next
    /// ```
    IndexSet(TermId<IndexSetTerm>),
    /// Runtime key membership check.
    ///
    /// ```ds
    /// "name" in value
    /// ```
    KeyMembership(TermId<KeyMembershipTerm>),
    /// Runtime nominal instance check.
    ///
    /// ```ds
    /// value instanceof Error
    /// ```
    InstanceCheck(TermId<InstanceCheckTerm>),
    /// Runtime identity equality check.
    ///
    /// ```ds
    /// left === right
    /// ```
    Identity(TermId<IdentityTerm>),
    /// Runtime await expression.
    ///
    /// ```ds
    /// await value
    /// ```
    Await(TermId<AwaitTerm>),
    /// Runtime try expression.
    ///
    /// ```ds
    /// value?
    /// ```
    Try(TermId<TryTerm>),
    /// Runtime yield expression.
    ///
    /// ```ds
    /// yield value
    /// ```
    Yield(TermId<YieldTerm>),
    /// Runtime try failure projection.
    ///
    /// ```ds
    /// try { value? } catch (error) { ... }
    /// ```
    TryFailure(TermId<TryFailureTerm>),
    /// Runtime template string expression.
    ///
    /// ```ds
    /// `hello ${name}`
    /// ```
    Template(TermId<TemplateTerm>),
    /// Runtime tagged template expression.
    ///
    /// ```ds
    /// sql`select ${id}`
    /// ```
    TaggedTemplate(TermId<TaggedTemplateTerm>),
}

impl TypeTerm {
    /// Return the unit type.
    pub(in crate::check) fn unit() -> Self {
        TypeTerm::Tuple {
            form: dir::TupleForm::Tuple,
            elements: Vec::new(),
        }
    }

    /// Return whether this is the unit type.
    pub(in crate::check) fn is_unit(&self) -> bool {
        matches!(
            self,
            TypeTerm::Tuple {
                form: dir::TupleForm::Tuple,
                elements,
            } if elements.is_empty()
        )
    }

    /// Return the structural key represented by this literal type.
    pub(in crate::check) fn static_key(&self) -> Option<dir::StaticKey> {
        match self {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::String(name))) => {
                Some(dir::StaticKey::Name(*name))
            }
            _ => None,
        }
    }

    /// Return whether this term can carry nominal identity.
    pub(in crate::check) fn can_be_nominal(&self) -> bool {
        matches!(
            self,
            Self::Reference { .. } | Self::Form { .. } | Self::Type(_)
        )
    }

    /// Return the nominal symbol rooted in this type term.
    pub(in crate::check) fn nominal_symbol(
        &self,
        state: &CheckState<'_>,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        match self {
            Self::Reference {
                origin: _,
                symbol,
                arguments: _,
            } if state.symbol_kind(*symbol).is_nominal() => Ok(Some(*symbol)),
            Self::Form { form: _, payload } => state.type_operand_nominal_symbol(*payload),
            Self::Type(ty) => state.type_nominal_symbol(*ty),
            _ => Ok(None),
        }
    }

    /// Return whether this term can be affected by one substitution.
    pub(in crate::check) fn needs_substitution(
        &self,
        substitution: &SubstitutionSet,
        state: &CheckState<'_>,
    ) -> CompilerResult<bool> {
        let needs_substitution = match self {
            Self::Parameter(parameter) => {
                substitution.has_generic(*parameter)
                    || state
                        .substitution_static_operand(substitution, *parameter)
                        .is_some()
            }
            Self::This => state.substitution_receiver_operand(substitution).is_some(),
            Self::Reference {
                origin: _,
                symbol,
                arguments,
            } => {
                state
                    .substitution_type_symbol_operand(substitution, *symbol)?
                    .is_some()
                    || arguments.iter().try_fold(false, |found, argument| {
                        if found {
                            return Ok(true);
                        }

                        argument.needs_substitution(substitution, state)
                    })?
            }
            Self::Form { form: _, payload }
            | Self::Dynamic {
                constraint: payload,
            } => payload.needs_substitution(substitution, state)?,
            Self::Array { element } | Self::Slice { element } => {
                element.needs_substitution(substitution, state)?
            }
            Self::FixedArray { element, length } => {
                element.needs_substitution(substitution, state)?
                    || length.needs_substitution(substitution, state)?
            }
            Self::Tuple { form: _, elements } => {
                elements.iter().try_fold(false, |found, element| {
                    if found {
                        return Ok(true);
                    }

                    element.ty.needs_substitution(substitution, state)
                })?
            }
            Self::Union { elements } | Self::Intersection { elements } => {
                elements.iter().try_fold(false, |found, element| {
                    if found {
                        return Ok(true);
                    }

                    element.needs_substitution(substitution, state)
                })?
            }
            Self::StaticValue { value } => value.needs_substitution(substitution, state)?,
            Self::Closure {
                function,
                environment,
            } => {
                function.needs_substitution(substitution, state)?
                    || environment.needs_substitution(substitution, state)?
            }
            Self::Literal(_)
            | Self::Intrinsic
            | Self::Type(_)
            | Self::Range { .. }
            | Self::ImportMeta(_) => false,
            Self::Member(_)
            | Self::Operation(_)
            | Self::Shape(_)
            | Self::Function(_)
            | Self::Call(_)
            | Self::Construct(_)
            | Self::RangeValue(_)
            | Self::Tree(_)
            | Self::TypeValue(_)
            | Self::Receiver(_)
            | Self::Super(_)
            | Self::Operator(_)
            | Self::Index(_)
            | Self::IndexSet(_)
            | Self::KeyMembership(_)
            | Self::InstanceCheck(_)
            | Self::Identity(_)
            | Self::Await(_)
            | Self::Try(_)
            | Self::Yield(_)
            | Self::TryFailure(_)
            | Self::Template(_)
            | Self::TaggedTemplate(_) => true,
        };

        Ok(needs_substitution)
    }

    /// Return the direct operand replacement for this term.
    pub(in crate::check) fn direct_substitution(
        &self,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Option<TypeOperand>> {
        // substitute type parameters directly to operands
        if let Self::Parameter(parameter) = self {
            if let Some(argument) = state.substitution_type_operand(substitution, *parameter) {
                return Ok(Some(argument));
            }
            if let Some(argument) = state.substitution_static_operand(substitution, *parameter) {
                let term = state
                    .inference
                    .push_term(Self::StaticValue { value: argument });

                return Ok(Some(term.into()));
            }
        }

        // substitute receiver placeholders directly to operands
        if matches!(self, Self::This)
            && let Some(receiver) = state.substitution_receiver_operand(substitution)
        {
            return Ok(Some(receiver));
        }

        Ok(None)
    }
}

impl CheckState<'_> {
    /// Return the nominal symbol rooted in one type operand.
    pub(in crate::check) fn type_operand_nominal_symbol(
        &self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(operand) = self.resolved_type_operand(operand) else {
            return Ok(None);
        };

        match operand {
            TypeOperand::Term(term) => self.inference.term(term).nominal_symbol(self),
            TypeOperand::Type(ty) => self.type_nominal_symbol(ty),
            TypeOperand::Variable(_) => Ok(None),
        }
    }

    /// Return the nominal symbol rooted in one committed type.
    pub(in crate::check) fn type_nominal_symbol(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbol = match self.r#type(ty) {
            dir::Type::Reference(reference) if self.symbol_kind(reference.symbol).is_nominal() => {
                Some(reference.symbol)
            }
            dir::Type::Form(form) => {
                return self.type_nominal_symbol(form.value);
            }
            _ => None,
        };

        Ok(symbol)
    }
}

impl CheckState<'_> {
    /// Return the element count when one variable is backed by an array literal.
    pub(in crate::check) fn variable_array_literal_length(
        &self,
        variable: VariableId,
    ) -> Option<usize> {
        let Origin::Node(node) = self.variable(variable).source else {
            return None;
        };
        if node.local_id.ty != dir::NodeType::Expression {
            return None;
        }
        let view = self.module(variable.module).view();
        let expression = dir::LocalNodeId::<dir::Expression>::new(node.local_id.id);
        let dir::Expression::ArrayExpression { elements } = view.tree().get(expression) else {
            return None;
        };

        Some(elements.len())
    }

    /// Decide one type term relation.
    pub(in crate::check) fn decide_type_term_relation(
        &mut self,
        module: Option<ModuleId>,
        relation: TypeRelation,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match relation {
            TypeRelation::Equal => self.decide_type_equal(left, right)?,
            TypeRelation::Assignable => self.decide_type_assignable(left, right)?,
            TypeRelation::Castable => self.decide_type_castable(left, right)?,
            TypeRelation::Satisfies => self.decide_type_satisfies(module, left, right)?,
            TypeRelation::Extends | TypeRelation::Implements => {
                self.decide_type_assignable(left, right)?
            }
        };

        Ok(decision)
    }

    /// Decide one solved type relation.
    pub(in crate::check) fn decide_type_relation(
        &mut self,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        let left = left.into();
        let right = right.into();
        if left == right {
            return Ok(Answer::Ready(true));
        }
        if matches!(relation, TypeRelation::Assignable | TypeRelation::Castable)
            && let Some(decision) = self.decide_array_literal_fixed_array_relation(left, right)?
        {
            return Ok(decision);
        }

        let Some(origin) = self
            .type_operand_origin(left)
            .or_else(|| self.type_operand_origin(right))
        else {
            return self.decide_type_operand_relation(None, relation, left, right);
        };

        let Answer::Ready(left) = self.reduce_type_operand(origin, left)? else {
            return Ok(Answer::pending(left.dependencies(self)));
        };
        let Answer::Ready(right) = self.reduce_type_operand(origin, right)? else {
            return Ok(Answer::pending(right.dependencies(self)));
        };

        self.decide_type_operand_relation(Some(origin.module()), relation, left, right)
    }

    /// Decide one type relation after reducing both operands.
    fn decide_type_operand_relation(
        &mut self,
        module: Option<ModuleId>,
        relation: TypeRelation,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Answer<bool>> {
        let Some(left) = self.type_operand_term_id(left)? else {
            return Ok(Answer::pending(left.dependencies(self)));
        };
        let Some(right) = self.type_operand_term_id(right)? else {
            return Ok(Answer::pending(right.dependencies(self)));
        };

        self.decide_type_term_id_relation(module, relation, left, right)
    }

    /// Return one term id for a reduced type operand.
    pub(in crate::check) fn type_operand_term_id(
        &mut self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TermId<TypeTerm>>> {
        let operand = match operand {
            TypeOperand::Variable(variable) => {
                let Some(operand) = self.resolved_type_variable(variable) else {
                    return Ok(None);
                };

                operand
            }
            TypeOperand::Term(_) | TypeOperand::Type(_) => operand,
        };

        match operand {
            TypeOperand::Term(term) => Ok(Some(term)),
            TypeOperand::Type(ty) => {
                let term = self.import_type_term(Origin::Type(ty), ty)?;
                let term = self.inference.push_term(term);

                Ok(Some(term))
            }
            TypeOperand::Variable(_) => Ok(None),
        }
    }

    /// Decide one type relation by term id.
    pub(in crate::check) fn decide_type_term_id_relation(
        &mut self,
        module: Option<ModuleId>,
        relation: TypeRelation,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        self.decide_type_term_relation(module, relation, left, right)
    }

    /// Return a source origin for one type term when it has one.
    fn type_term_origin(&self, term: &TypeTerm) -> Option<Origin> {
        match term {
            TypeTerm::Type(ty) => Some(Origin::Type(*ty)),
            TypeTerm::Parameter(parameter) => self.generic_parameter_origin(*parameter),
            TypeTerm::Reference { origin, .. } => Some(*origin),
            TypeTerm::Member(member) => {
                let member = self.inference.term(*member);

                Some(member.origin)
            }
            TypeTerm::Form { payload, .. }
            | TypeTerm::Dynamic {
                constraint: payload,
            }
            | TypeTerm::Array { element: payload }
            | TypeTerm::Slice { element: payload } => self.type_operand_origin(*payload),
            TypeTerm::StaticValue { value } => self.static_operand_origin(*value),
            TypeTerm::FixedArray { element, .. } => self.type_operand_origin(*element),
            TypeTerm::Tuple { elements, .. } => elements
                .iter()
                .find_map(|element| self.type_operand_origin(element.ty)),
            TypeTerm::Shape(shape) => self
                .inference
                .term(*shape)
                .members
                .iter()
                .find_map(|member| self.shape_member_origin(member)),
            TypeTerm::Function(function) => self.function_origin(self.inference.term(*function)),
            TypeTerm::Closure {
                function,
                environment,
            } => self
                .type_operand_origin(*function)
                .or_else(|| self.type_operand_origin(*environment)),
            TypeTerm::Union { elements } | TypeTerm::Intersection { elements } => elements
                .iter()
                .find_map(|operand| self.type_operand_origin(*operand)),
            TypeTerm::Operation(operation) => {
                self.type_operation_origin(self.inference.term(*operation))
            }
            TypeTerm::Call(call) => Some(Origin::Node(self.inference.term(*call).source)),
            TypeTerm::Construct(construct) => {
                Some(Origin::Node(self.inference.term(*construct).source))
            }
            TypeTerm::Operator(operator) => {
                Some(Origin::Node(self.inference.term(*operator).source))
            }
            TypeTerm::Index(index) => Some(Origin::Node(self.inference.term(*index).source)),
            TypeTerm::IndexSet(set) => Some(Origin::Node(self.inference.term(*set).source)),
            TypeTerm::InstanceCheck(instance) => {
                Some(Origin::Node(self.inference.term(*instance).source))
            }
            TypeTerm::Identity(identity) => {
                Some(Origin::Node(self.inference.term(*identity).source))
            }
            TypeTerm::Receiver(receiver) => {
                Some(Origin::Node(self.inference.term(*receiver).source))
            }
            TypeTerm::Super(super_term) => {
                Some(Origin::Node(self.inference.term(*super_term).source))
            }
            TypeTerm::RangeValue(range) => Some(Origin::Node(self.inference.term(*range).source)),
            TypeTerm::Tree(tree) => Some(Origin::Node(self.inference.term(*tree).source)),
            TypeTerm::TypeValue(value) => Some(Origin::Node(self.inference.term(*value).source)),
            TypeTerm::ImportMeta(meta) => Some(Origin::Node(self.inference.term(*meta).source)),
            TypeTerm::KeyMembership(membership) => {
                Some(Origin::Node(self.inference.term(*membership).source))
            }
            TypeTerm::Await(awaited) => Some(Origin::Node(self.inference.term(*awaited).source)),
            TypeTerm::Try(tried) => Some(Origin::Node(self.inference.term(*tried).source)),
            TypeTerm::Yield(yielded) => Some(Origin::Node(self.inference.term(*yielded).source)),
            TypeTerm::TryFailure(tried) => Some(Origin::Node(self.inference.term(*tried).source)),
            TypeTerm::Template(template) => {
                Some(Origin::Node(self.inference.term(*template).source))
            }
            TypeTerm::TaggedTemplate(template) => {
                Some(Origin::Node(self.inference.term(*template).source))
            }
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::This
            | TypeTerm::Range { .. } => None,
        }
    }

    /// Return a source origin for one type operand when it has one.
    fn type_operand_origin(&self, operand: TypeOperand) -> Option<Origin> {
        match operand {
            TypeOperand::Variable(variable) => Some(self.variable(variable).source),
            TypeOperand::Term(term) => self.type_term_origin(self.inference.term(term)),
            TypeOperand::Type(ty) => Some(Origin::Type(ty)),
        }
    }

    /// Return a source origin for one static operand when it has one.
    fn static_operand_origin(&self, operand: StaticOperand) -> Option<Origin> {
        match operand {
            StaticOperand::Variable(variable) => Some(self.variable(variable).source),
            StaticOperand::Term(_) | StaticOperand::Static(_) => None,
        }
    }

    /// Return a source origin for one shape member when it has one.
    fn shape_member_origin(&self, member: &ShapeMember) -> Option<Origin> {
        match member {
            ShapeMember::Field { ty, .. }
            | ShapeMember::CallSignature { ty }
            | ShapeMember::ConstructSignature { ty } => self.type_operand_origin(*ty),
            ShapeMember::Spread { origin, .. } => Some(*origin),
            ShapeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => self
                .type_operand_origin(*key_type)
                .or_else(|| self.type_operand_origin(*value_type)),
        }
    }

    /// Return a source origin for one function term when it has one.
    fn function_origin(&self, function: &FunctionTerm) -> Option<Origin> {
        function
            .this_parameter
            .and_then(|parameter| self.type_operand_origin(parameter))
            .or_else(|| {
                function
                    .parameters
                    .iter()
                    .find_map(|parameter| self.type_operand_origin(parameter.ty))
            })
            .or_else(|| {
                function
                    .return_type
                    .and_then(|return_type| self.type_operand_origin(return_type))
            })
    }

    /// Return a source origin for one type operation when it has one.
    fn type_operation_origin(&self, operation: &TypeOperationTerm) -> Option<Origin> {
        match operation {
            TypeOperationTerm::StringMapping { argument, .. }
            | TypeOperationTerm::KeyOf { target: argument }
            | TypeOperationTerm::Widen { source: argument } => self.type_operand_origin(*argument),
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => self
                .type_operand_origin(*left)
                .or_else(|| self.type_operand_origin(*right))
                .or_else(|| self.type_operand_origin(*then_type))
                .or_else(|| self.type_operand_origin(*else_type)),
            TypeOperationTerm::Mapped {
                parameter, value, ..
            } => self
                .type_operand_origin(parameter.constraint)
                .or_else(|| {
                    parameter
                        .key_remap
                        .and_then(|key| self.type_operand_origin(key))
                })
                .or_else(|| self.type_operand_origin(*value)),
            TypeOperationTerm::Index { left, index } => self
                .type_operand_origin(*left)
                .or_else(|| self.type_operand_origin(*index)),
            TypeOperationTerm::TemplateLiteral { spans, .. } => spans
                .iter()
                .find_map(|span| self.type_operand_origin(*span)),
            TypeOperationTerm::Infer { constraint, .. } => {
                constraint.and_then(|constraint| self.type_operand_origin(constraint))
            }
            TypeOperationTerm::BestCommon { elements } => elements
                .iter()
                .find_map(|element| self.type_operand_origin(*element)),
            TypeOperationTerm::Exclude { source, target }
            | TypeOperationTerm::Extract { source, target } => self
                .type_operand_origin(*source)
                .or_else(|| self.type_operand_origin(*target)),
            TypeOperationTerm::Intrinsic { arguments, .. } => arguments
                .iter()
                .find_map(|argument| self.generic_argument_origin(argument)),
        }
    }

    /// Return a source origin for one generic argument when it has one.
    fn generic_argument_origin(&self, argument: &GenericArgument) -> Option<Origin> {
        match argument {
            GenericArgument::Type(operand)
            | GenericArgument::SpreadType(operand)
            | GenericArgument::AssociatedType { value: operand, .. } => {
                self.type_operand_origin(*operand)
            }
            GenericArgument::Static(operand)
            | GenericArgument::SpreadStatic(operand)
            | GenericArgument::AssociatedConst { value: operand, .. } => {
                self.static_operand_origin(*operand)
            }
            GenericArgument::TypeOrStatic { source }
            | GenericArgument::SpreadTypeOrStatic { source } => {
                self.type_operand_origin(source.ty).or_else(|| {
                    source
                        .r#static
                        .and_then(|value| self.static_operand_origin(value))
                })
            }
        }
    }

    /// Return the source origin for one generic parameter.
    fn generic_parameter_origin(&self, parameter: GenericParameterId) -> Option<Origin> {
        let parameter = self.inference.generic_parameter(parameter)?;

        match parameter.parameter().key {
            dir::GenericParameterKey::Symbol(symbol) => Some(Origin::Symbol(symbol)),
            dir::GenericParameterKey::Generated(_) => {
                let template = self
                    .inference
                    .generic_template(parameter.parameter().template)?;

                Some(Origin::Node(template.source))
            }
        }
    }

    /// Decide array literal assignability into a fixed array target.
    fn decide_array_literal_fixed_array_relation(
        &mut self,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let Some(source_variable) = source.variable() else {
            return Ok(None);
        };
        if self
            .variable_array_literal_length(source_variable)
            .is_none()
        {
            return Ok(None);
        }

        let Some(source) = self.type_operand_term_id(source)? else {
            return Ok(None);
        };
        let TypeTerm::Array {
            element: source_element,
        } = self.inference.term(source)
        else {
            return Ok(None);
        };
        let source_element = *source_element;

        let Some(target) = self.type_operand_term_id(target)? else {
            return Ok(None);
        };
        let TypeTerm::FixedArray {
            element: target_element,
            length: target_length,
        } = self.inference.term(target)
        else {
            return Ok(None);
        };
        let target_element = *target_element;
        let target_length = *target_length;

        let Some(length) = self.decide_array_literal_length(source_variable, target_length)? else {
            return Ok(Some(Answer::pending(target_length.dependencies(self))));
        };
        if length != Answer::Ready(true) {
            return Ok(Some(length));
        }

        let element =
            self.decide_type_relation(TypeRelation::Assignable, source_element, target_element)?;

        Ok(Some(element))
    }

    /// Decide whether one array literal length fits a static length target.
    fn decide_array_literal_length(
        &self,
        variable: VariableId,
        target: StaticOperand,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let Some(length) = self.variable_array_literal_length(variable) else {
            return Ok(None);
        };
        let Some(target) = self.resolved_static_operand(target) else {
            return Ok(Some(Answer::pending(target.dependencies(self))));
        };
        let length = StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(length as i64),
        });
        let decision = match target {
            StaticOperand::Term(target) => {
                let target = self.inference.term(target);

                self.decide_static_term_relation(StaticRelation::Equal, &length, target)?
            }
            StaticOperand::Static(target) => {
                let target = StaticTerm::Static(target);

                self.decide_static_term_relation(StaticRelation::Equal, &length, &target)?
            }
            StaticOperand::Variable(variable) => Answer::pending([Dependency::Variable(variable)]),
        };

        Ok(Some(decision))
    }

    /// Reduce one owned type term when the solver has enough input.
    pub(in crate::check) fn reduce_type_term(
        &mut self,
        origin: Origin,
        term: TypeTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let term = self.inference.push_term(term);

        self.reduce_type_operand(origin, term.into())
    }

    /// Reduce one interned type term when the solver has enough input.
    pub(in crate::check) fn reduce_type_term_by_id(
        &mut self,
        origin: Origin,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Answer::Ready(reduced) = self.reduce_type_term_once(origin, term)? else {
            return Ok(Answer::pending(TypeOperand::Term(term).dependencies(self)));
        };

        Ok(Answer::Ready(reduced))
    }

    /// Intern one type term as a type operand.
    pub(in crate::check) fn type_term_operand(&mut self, term: TypeTerm) -> TypeOperand {
        match term {
            TypeTerm::Type(ty) => TypeOperand::Type(ty),
            term => self.inference.push_term(term).into(),
        }
    }

    /// Return one reduction step for one type term.
    fn reduce_type_term_once(
        &mut self,
        origin: Origin,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let module = origin.module();
        // committed type
        if let TypeTerm::Type(ty) = self.inference.term(term) {
            return Ok(Answer::Ready(TypeOperand::Type(*ty)));
        }

        // member projection
        let member = match self.inference.term(term) {
            TypeTerm::Member(member) => Some(*member),
            _ => None,
        };
        if let Some(member) = member {
            let member = self.inference.term(member);
            let member_origin = member.origin;
            let receiver = member.receiver.clone();
            let key = member.key;
            let arguments = member
                .arguments
                .iter()
                .copied()
                .collect::<SmallVec<[GenericArgument; 2]>>();
            let projection = TypeOperand::Term(term);

            return self.reduce_member_term(
                module,
                member_origin,
                receiver,
                key,
                &arguments,
                Some(projection),
            );
        }

        // type operation
        let operation = match self.inference.term(term) {
            TypeTerm::Operation(operation) => Some(*operation),
            _ => None,
        };
        if let Some(operation) = operation {
            return self.reduce_type_operation_term(origin, module, term, operation);
        }

        // declaration reference
        let reference = match self.inference.term(term) {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => Some((
                *symbol,
                arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>(),
            )),
            _ => None,
        };
        if let Some((symbol, arguments)) = reference {
            let reduction = self.reduce_reference_term(origin, term, symbol, &arguments)?;

            return Ok(reduction);
        }

        // static value
        let static_value = match self.inference.term(term) {
            TypeTerm::StaticValue { value } => Some(*value),
            _ => None,
        };
        if let Some(value) = static_value {
            let reduction = self.reduce_static_value_term(module, value)?;

            return Ok(reduction.map(|term| self.type_term_operand(term)));
        }

        // reduce form payload
        let form = match self.inference.term(term) {
            TypeTerm::Form { form, payload } => Some((*form, *payload)),
            _ => None,
        };
        if let Some((form, previous)) = form {
            let payload = previous;
            let Answer::Ready(payload) = self.reduce_type_operand(origin, payload)? else {
                return Ok(Answer::pending(payload.dependencies(self)));
            };
            if payload == previous {
                return Ok(Answer::Ready(TypeOperand::Term(term)));
            }

            let term = self.type_term_operand(TypeTerm::Form { form, payload });

            return Ok(Answer::Ready(term));
        }

        // reducible payload terms
        if let Some(reduction) = self.reduce_wrapped_type_term(origin, module, term)? {
            return Ok(reduction);
        }

        Ok(Answer::Ready(TypeOperand::Term(term)))
    }

    /// Reduce one type operation term.
    fn reduce_type_operation_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: TermId<TypeTerm>,
        operation: TermId<TypeOperationTerm>,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let operation_term = self.inference.term(operation);
        match operation_term {
            TypeOperationTerm::Exclude { source, target } => {
                let source = *source;
                let target = *target;

                self.reduce_exclude_term(origin, source, target)
            }
            TypeOperationTerm::Extract { source, target } => {
                let source = *source;
                let target = *target;

                self.reduce_extract_term(origin, source, target)
            }
            TypeOperationTerm::BestCommon { elements } => {
                let elements = elements
                    .iter()
                    .copied()
                    .collect::<SmallVec<[TypeOperand; 4]>>();

                self.reduce_best_common_term(&elements)
            }
            TypeOperationTerm::Widen { source } => {
                let source = *source;

                self.reduce_widen_term(origin, source)
            }
            TypeOperationTerm::Intrinsic { item, arguments } => {
                let item = *item;
                let arguments = arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();

                self.reduce_memory_term(module, item, &arguments)
            }
            TypeOperationTerm::StringMapping { mapping, argument } => {
                let mapping = *mapping;
                let argument = *argument;

                self.reduce_string_mapping_term(origin, module, mapping, argument)
            }
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                let left = *left;
                let right = *right;
                let then_type = *then_type;
                let else_type = *else_type;

                self.reduce_conditional_term(left, right, then_type, else_type)
            }
            TypeOperationTerm::Index { left, index } => {
                let left = *left;
                let index = *index;

                self.reduce_type_index_term(origin, module, left, index)
            }
            TypeOperationTerm::TemplateLiteral { .. }
            | TypeOperationTerm::Infer { .. }
            | TypeOperationTerm::KeyOf { .. }
            | TypeOperationTerm::Mapped { .. } => Ok(Answer::Ready(TypeOperand::Term(term))),
        }
    }

    /// Reduce one term backed by a nested payload arena.
    fn reduce_wrapped_type_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Option<Answer<TypeOperand>>> {
        match self.inference.term(term) {
            TypeTerm::Call(call) => {
                let reduction = self.reduce_call_term(origin, module, *call)?;

                Ok(Some(reduction))
            }
            TypeTerm::Construct(construct) => {
                let reduction = self.reduce_construct_term(origin, module, *construct)?;

                Ok(Some(reduction))
            }
            TypeTerm::RangeValue(range) => {
                let reduction = self.reduce_range_value_term(*self.inference.term(*range))?;

                Ok(Some(reduction))
            }
            TypeTerm::Tree(tree) => {
                let reduction = self.reduce_tree_term(*tree)?;

                Ok(Some(reduction))
            }
            TypeTerm::TypeValue(value) => {
                let reduction = self.reduce_type_value_term(*self.inference.term(*value))?;

                Ok(Some(reduction))
            }
            TypeTerm::ImportMeta(meta) => {
                let reduction = self.reduce_import_meta_term(*self.inference.term(*meta))?;

                Ok(Some(reduction))
            }
            TypeTerm::Receiver(receiver) => {
                let reduction = self.reduce_receiver_term(*self.inference.term(*receiver))?;

                Ok(Some(reduction))
            }
            TypeTerm::Super(term) => {
                let reduction = self.reduce_super_term(*self.inference.term(*term))?;

                Ok(Some(reduction))
            }
            TypeTerm::Operator(operator) => {
                let reduction =
                    self.reduce_operator_term(origin, *self.inference.term(*operator))?;

                Ok(Some(reduction))
            }
            TypeTerm::Index(index) => {
                let reduction =
                    self.reduce_index_term(origin, module, *self.inference.term(*index))?;

                Ok(Some(reduction))
            }
            TypeTerm::IndexSet(set) => {
                let reduction =
                    self.reduce_index_set_term(origin, module, *self.inference.term(*set))?;

                Ok(Some(reduction))
            }
            TypeTerm::KeyMembership(membership) => {
                let reduction =
                    self.reduce_key_membership_term(*self.inference.term(*membership))?;

                Ok(Some(reduction))
            }
            TypeTerm::InstanceCheck(instance) => {
                let reduction = self.reduce_instance_check_term(*self.inference.term(*instance))?;

                Ok(Some(reduction))
            }
            TypeTerm::Identity(identity) => {
                let identity = *self.inference.term(*identity);
                let reduction = self.reduce_identity_term(&identity)?;

                Ok(Some(reduction))
            }
            TypeTerm::Template(template) => {
                let reduction = self.reduce_template_term(*template)?;

                Ok(Some(reduction))
            }
            TypeTerm::TaggedTemplate(template) => {
                let reduction = self.reduce_tagged_template_term(origin, *template)?;

                Ok(Some(reduction))
            }
            TypeTerm::Await(awaited) => {
                let reduction = self.reduce_await_term(*self.inference.term(*awaited))?;

                Ok(Some(reduction))
            }
            TypeTerm::Try(tried) => Ok(Some(self.reduce_try_term(
                origin,
                module,
                *self.inference.term(*tried),
            )?)),
            TypeTerm::Yield(yielded) => {
                let reduction = self.reduce_yield_term(*self.inference.term(*yielded))?;

                Ok(Some(reduction))
            }
            TypeTerm::TryFailure(tried) => Ok(Some(self.reduce_try_failure_term(
                origin,
                module,
                *self.inference.term(*tried),
            )?)),
            _ => Ok(None),
        }
    }

    /// Constrain solved type operands to be equal.
    pub(in crate::check) fn constrain_type_operands_equal(
        &mut self,
        origin: Origin,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<()> {
        let Some(left) = self.type_operand_term_id(left)? else {
            return Ok(());
        };
        let Some(right) = self.type_operand_term_id(right)? else {
            return Ok(());
        };

        self.constrain_type_term_ids_equal(origin, left, right)
    }

    /// Constrain solved type term ids to be equal.
    fn constrain_type_term_ids_equal(
        &mut self,
        origin: Origin,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<()> {
        if {
            let left = self.inference.term(left);
            let right = self.inference.term(right);

            matches!(left, TypeTerm::Literal(TypeLiteralTerm::Void)) && right.is_unit()
                || matches!(right, TypeTerm::Literal(TypeLiteralTerm::Void)) && left.is_unit()
        } {
            return Ok(());
        }

        if let Some((left_form, right_form, left_value, right_value)) = {
            let left = self.inference.term(left);
            let right = self.inference.term(right);

            match (left, right) {
                (
                    TypeTerm::Form {
                        form: left_form,
                        payload: left_value,
                    },
                    TypeTerm::Form {
                        form: right_form,
                        payload: right_value,
                    },
                ) => Some((*left_form, *right_form, *left_value, *right_value)),
                _ => None,
            }
        } {
            let left_form = *self.inference.term(left_form);
            let right_form = *self.inference.term(right_form);

            self.constrain_form_equal(origin, &left_form, &right_form)?;
            self.constrain_type(
                origin,
                TypeRelation::Equal,
                left_value,
                right_value,
                Condition::Always,
            );

            return Ok(());
        }

        if let Some((left_element, right_element)) = {
            let left = self.inference.term(left);
            let right = self.inference.term(right);

            match (left, right) {
                (
                    TypeTerm::Array {
                        element: left_element,
                    },
                    TypeTerm::Array {
                        element: right_element,
                    },
                )
                | (
                    TypeTerm::Slice {
                        element: left_element,
                    },
                    TypeTerm::Slice {
                        element: right_element,
                    },
                ) => Some((*left_element, *right_element)),
                _ => None,
            }
        } {
            self.constrain_type(
                origin,
                TypeRelation::Equal,
                left_element,
                right_element,
                Condition::Always,
            );

            return Ok(());
        }

        if let Some((left_element, right_element, left_length, right_length)) = {
            let left = self.inference.term(left);
            let right = self.inference.term(right);

            match (left, right) {
                (
                    TypeTerm::FixedArray {
                        element: left_element,
                        length: left_length,
                    },
                    TypeTerm::FixedArray {
                        element: right_element,
                        length: right_length,
                    },
                ) => Some((*left_element, *right_element, *left_length, *right_length)),
                _ => None,
            }
        } {
            self.constrain_type(
                origin,
                TypeRelation::Equal,
                left_element,
                right_element,
                Condition::Always,
            );
            self.constrain_static(
                origin,
                StaticRelation::Equal,
                left_length,
                right_length,
                Condition::Always,
            );

            return Ok(());
        }

        if self.constrain_tuple_terms_equal(origin, left, right)? {
            return Ok(());
        }

        if let Some((left_shape, right_shape)) = {
            let left = self.inference.term(left);
            let right = self.inference.term(right);

            match (left, right) {
                (TypeTerm::Shape(left), TypeTerm::Shape(right)) => Some((*left, *right)),
                _ => None,
            }
        } {
            self.constrain_shape_terms_equal(origin, left_shape, right_shape)?;

            return Ok(());
        }

        if {
            let left = self.inference.term(left);
            let right = self.inference.term(right);

            match (left, right) {
                (
                    TypeTerm::Reference {
                        origin: _,
                        symbol: left_symbol,
                        arguments: left_arguments,
                    },
                    TypeTerm::Reference {
                        origin: _,
                        symbol: right_symbol,
                        arguments: right_arguments,
                    },
                ) => left_symbol == right_symbol && left_arguments.len() == right_arguments.len(),
                _ => false,
            }
        } {
            self.constrain_reference_arguments_equal(origin, left, right)?;
        }

        Ok(())
    }

    /// Constrain solved type operands to be assignable.
    pub(in crate::check) fn constrain_type_operands_assignable(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<()> {
        let source_operand = self.contextual_type_operand(origin, source)?;
        let target_operand = self.contextual_type_operand(origin, target)?;
        let Some(source) = self.type_operand_term_id(source_operand)? else {
            return Ok(());
        };
        let Some(target) = self.type_operand_term_id(target_operand)? else {
            return Ok(());
        };

        self.constrain_type_term_ids_assignable(
            origin,
            source_operand,
            source,
            target_operand,
            target,
        )
    }

    /// Constrain solved type term ids to be assignable.
    fn constrain_type_term_ids_assignable(
        &mut self,
        origin: Origin,
        source_operand: TypeOperand,
        source: TermId<TypeTerm>,
        target_operand: TypeOperand,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<()> {
        if {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            matches!(source, TypeTerm::Literal(TypeLiteralTerm::Void)) && target.is_unit()
                || matches!(target, TypeTerm::Literal(TypeLiteralTerm::Void)) && source.is_unit()
        } {
            return Ok(());
        }

        if let Some((source_form, target_form, source_value, target_value)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (
                    TypeTerm::Form {
                        form: source_form,
                        payload: source_value,
                    },
                    TypeTerm::Form {
                        form: target_form,
                        payload: target_value,
                    },
                ) => Some((*source_form, *target_form, *source_value, *target_value)),
                _ => None,
            }
        } {
            let source_form = *self.inference.term(source_form);
            let target_form = *self.inference.term(target_form);

            self.constrain_form_assignable(origin, &source_form, &target_form)?;
            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                source_value,
                target_value,
                Condition::Always,
            );

            return Ok(());
        }

        if {
            let source = self.inference.term(source);

            matches!(source, TypeTerm::Union { .. })
        } {
            let source_union = source;
            let len = self.union_term_len(source_union);

            // constrain each source variant to the target
            for index in 0..len {
                let Some(source) = self.union_term_element(source_union, index) else {
                    return Ok(());
                };

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    source,
                    target_operand,
                    Condition::Always,
                );
            }

            return Ok(());
        }

        if {
            let target = self.inference.term(target);

            matches!(target, TypeTerm::Union { .. })
        } {
            if let Some(target) = self.single_viable_union_target(origin, source_operand, target)? {
                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    source_operand,
                    target,
                    Condition::Always,
                );
            }

            return Ok(());
        }

        if let Some((source_element, target_element)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (
                    TypeTerm::Array {
                        element: source_element,
                    },
                    TypeTerm::Array {
                        element: target_element,
                    },
                )
                | (
                    TypeTerm::Array {
                        element: source_element,
                    },
                    TypeTerm::Slice {
                        element: target_element,
                    },
                )
                | (
                    TypeTerm::Slice {
                        element: source_element,
                    },
                    TypeTerm::Slice {
                        element: target_element,
                    },
                )
                | (
                    TypeTerm::FixedArray {
                        element: source_element,
                        length: _,
                    },
                    TypeTerm::Slice {
                        element: target_element,
                    },
                ) => Some((*source_element, *target_element)),
                _ => None,
            }
        } {
            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                source_element,
                target_element,
                Condition::Always,
            );

            return Ok(());
        }

        if let Some((source_element, target_element, source_length, target_length)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (
                    TypeTerm::FixedArray {
                        element: source_element,
                        length: source_length,
                    },
                    TypeTerm::FixedArray {
                        element: target_element,
                        length: target_length,
                    },
                ) => Some((
                    *source_element,
                    *target_element,
                    *source_length,
                    *target_length,
                )),
                _ => None,
            }
        } {
            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                source_element,
                target_element,
                Condition::Always,
            );
            self.constrain_static(
                origin,
                StaticRelation::Equal,
                source_length,
                target_length,
                Condition::Always,
            );

            return Ok(());
        }

        if self.constrain_tuple_terms_assignable(origin, source, target)? {
            return Ok(());
        }

        if let Some((source_shape, target_shape)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (TypeTerm::Shape(source), TypeTerm::Shape(target)) => Some((*source, *target)),
                _ => None,
            }
        } {
            self.constrain_shape_terms_assignable(origin, source_shape, target_shape)?;

            return Ok(());
        }

        if let Some((source_members, symbol, arguments)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (
                    TypeTerm::Shape(source),
                    TypeTerm::Reference {
                        origin: _,
                        symbol,
                        arguments,
                    },
                ) => Some((
                    self.inference
                        .term(*source)
                        .members
                        .iter()
                        .copied()
                        .collect::<SmallVec<[ShapeMember; 4]>>(),
                    *symbol,
                    arguments
                        .iter()
                        .copied()
                        .collect::<SmallVec<[GenericArgument; 2]>>(),
                )),
                _ => None,
            }
        } {
            self.constrain_shape_assignable_to_interface(
                origin,
                &source_members,
                symbol,
                &arguments,
            )?;

            return Ok(());
        }

        if let Some((symbol, arguments, target_members)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (
                    TypeTerm::Reference {
                        origin: _,
                        symbol,
                        arguments,
                    },
                    TypeTerm::Shape(target),
                ) => Some((
                    *symbol,
                    arguments
                        .iter()
                        .copied()
                        .collect::<SmallVec<[GenericArgument; 2]>>(),
                    self.inference
                        .term(*target)
                        .members
                        .iter()
                        .copied()
                        .collect::<SmallVec<[ShapeMember; 4]>>(),
                )),
                _ => None,
            }
        } {
            self.constrain_reference_assignable_to_shape(
                origin,
                symbol,
                &arguments,
                &target_members,
            )?;

            return Ok(());
        }

        if {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (
                    TypeTerm::Reference {
                        origin: _,
                        symbol: source_symbol,
                        arguments: source_arguments,
                    },
                    TypeTerm::Reference {
                        origin: _,
                        symbol: target_symbol,
                        arguments: target_arguments,
                    },
                ) => {
                    source_symbol == target_symbol
                        && source_arguments.len() == target_arguments.len()
                }
                _ => false,
            }
        } {
            self.constrain_reference_arguments_equal(origin, source, target)?;

            return Ok(());
        }

        if let Some((source_function, target_function)) = {
            let source = self.inference.term(source);
            let target = self.inference.term(target);

            match (source, target) {
                (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                    Some((*source, *target))
                }
                _ => None,
            }
        } {
            self.expect_function_term(origin, source_function, target_function)?;
        }

        Ok(())
    }

    /// Constrain two tuple terms by exact equality.
    fn constrain_tuple_terms_equal(
        &mut self,
        origin: Origin,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<bool> {
        let Some((left_len, right_len)) = self.equal_tuple_term_lengths(left, right) else {
            return Ok(false);
        };
        if left_len != right_len {
            return Ok(true);
        }

        // constrain one copied element pair at a time
        for index in 0..left_len {
            let Some((left, right)) = self.tuple_term_elements(left, right, index) else {
                return Ok(false);
            };

            self.constrain_type(
                origin,
                TypeRelation::Equal,
                left.ty,
                right.ty,
                Condition::Always,
            );
        }

        Ok(true)
    }

    /// Constrain two tuple terms by assignability.
    fn constrain_tuple_terms_assignable(
        &mut self,
        origin: Origin,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<bool> {
        let Some((source_len, target_len)) = self.equal_tuple_term_lengths(source, target) else {
            return Ok(false);
        };
        if source_len != target_len {
            return Ok(true);
        }

        // constrain one copied element pair at a time
        for index in 0..source_len {
            let Some((source, target)) = self.tuple_term_elements(source, target, index) else {
                return Ok(false);
            };

            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                source.ty,
                target.ty,
                Condition::Always,
            );
        }

        Ok(true)
    }

    /// Return tuple element lengths when two terms are same-form tuples.
    fn equal_tuple_term_lengths(
        &self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> Option<(usize, usize)> {
        match (self.inference.term(left), self.inference.term(right)) {
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left_elements,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right_elements,
                },
            ) if left_form == right_form => Some((left_elements.len(), right_elements.len())),
            _ => None,
        }
    }

    /// Return one copied tuple element pair.
    fn tuple_term_elements(
        &self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
        index: usize,
    ) -> Option<(TupleElement, TupleElement)> {
        match (self.inference.term(left), self.inference.term(right)) {
            (
                TypeTerm::Tuple { elements: left, .. },
                TypeTerm::Tuple {
                    elements: right, ..
                },
            ) => Some((*left.get(index)?, *right.get(index)?)),
            _ => None,
        }
    }

    /// Decide exact equality for two tuple terms.
    fn decide_tuple_terms_equal(
        &mut self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let Some((left_len, right_len)) = self.equal_tuple_term_lengths(left, right) else {
            return Ok(Answer::Ready(false));
        };
        if left_len != right_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // decide one copied element pair at a time
        for index in 0..left_len {
            let Some((left, right)) = self.tuple_term_elements(left, right, index) else {
                return Ok(Answer::Ready(false));
            };

            decision = decision.and(self.decide_tuple_element_equal(&left, &right)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide assignability for two tuple terms.
    fn decide_tuple_terms_assignable(
        &mut self,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let Some((source_len, target_len)) = self.equal_tuple_term_lengths(source, target) else {
            return Ok(Answer::Ready(false));
        };
        if source_len != target_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // decide one copied element pair at a time
        for index in 0..source_len {
            let Some((source, target)) = self.tuple_term_elements(source, target, index) else {
                return Ok(Answer::Ready(false));
            };

            decision = decision.and(self.decide_tuple_element_assignable(&source, &target)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}

impl TypeTerm {
    /// Substitute generic arguments through this type term.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let term = match self {
            TypeTerm::Parameter(parameter_id) => {
                if let Some(argument) = state.substitution_type_operand(substitution, *parameter_id)
                {
                    return Ok(Some(argument));
                } else if let Some(argument) =
                    state.substitution_static_operand(substitution, *parameter_id)
                {
                    TypeTerm::StaticValue { value: argument }
                } else {
                    self.clone()
                }
            }
            TypeTerm::StaticValue { value } => TypeTerm::StaticValue {
                value: state.substitute_static_operand(module, substitution, *value)?,
            },
            TypeTerm::Form { form, payload } => TypeTerm::Form {
                form: {
                    let form = state.inference.term(*form).clone();
                    let form = form.substitute(module, substitution, state)?;
                    state.inference.push_term(form)
                },
                payload: state.substitute_type_operand(module, substitution, *payload)?,
            },
            TypeTerm::Reference {
                origin: reference_origin,
                symbol,
                arguments,
            } => {
                if arguments.is_empty()
                    && let Some(argument) =
                        state.substitution_type_symbol_operand(substitution, *symbol)?
                {
                    return Ok(Some(argument));
                } else {
                    TypeTerm::Reference {
                        origin: *reference_origin,
                        symbol: *symbol,
                        arguments: state
                            .substitute_arguments(module, substitution, arguments)?
                            .into_iter()
                            .collect(),
                    }
                }
            }
            TypeTerm::Array { element } => TypeTerm::Array {
                element: state.substitute_type_operand(module, substitution, *element)?,
            },
            TypeTerm::Member(member) => {
                let member = state.inference.term(*member).clone();
                let member = member.substitute(module, substitution, state)?;
                let member = state.inference.push_term(member);

                TypeTerm::Member(member)
            }
            TypeTerm::FixedArray { element, length } => TypeTerm::FixedArray {
                element: state.substitute_type_operand(module, substitution, *element)?,
                length: state.substitute_static_operand(module, substitution, *length)?,
            },
            TypeTerm::Slice { element } => TypeTerm::Slice {
                element: state.substitute_type_operand(module, substitution, *element)?,
            },
            TypeTerm::Tuple { form, elements } => TypeTerm::Tuple {
                form: *form,
                elements: state
                    .substitute_tuple_elements(module, substitution, elements)?
                    .into(),
            },
            TypeTerm::Shape(shape) => {
                let members = state
                    .inference
                    .term(*shape)
                    .members
                    .iter()
                    .copied()
                    .collect::<SmallVec<[ShapeMember; 4]>>();
                let members = state
                    .substitute_shape_members(module, substitution, &members)?
                    .into();

                state.push_shape_type(members)
            }
            TypeTerm::Function(function) => {
                let function = state.inference.term(*function).clone();
                let function = function.substitute(module, substitution, state)?;
                let function = state.inference.push_term(function);

                TypeTerm::Function(function)
            }
            TypeTerm::Union { elements } => TypeTerm::Union {
                elements: state.substitute_type_operands(module, substitution, elements)?,
            },
            TypeTerm::Intersection { elements } => TypeTerm::Intersection {
                elements: state.substitute_type_operands(module, substitution, elements)?,
            },
            TypeTerm::Operation(operation) => {
                let operation = state.inference.term(*operation).clone();
                let operation = operation.substitute(module, substitution, state)?;
                let operation = state.inference.push_term(operation);

                TypeTerm::Operation(operation)
            }
            TypeTerm::Call(call) => {
                let call = state.inference.term(*call).clone();
                let callee = match call.callee {
                    CallCallee::Expression(callee) => CallCallee::Expression(
                        state.substitute_type_operand(module, substitution, callee)?,
                    ),
                    CallCallee::Reference { value, symbol } => CallCallee::Reference {
                        value: state.substitute_type_operand(module, substitution, value)?,
                        symbol,
                    },
                    CallCallee::Member(member) => {
                        let member = state.inference.term(member).clone();
                        let origin = match member.origin {
                            MemberProjectionOrigin::Expression { source } => {
                                MemberProjectionOrigin::Expression { source }
                            }
                            MemberProjectionOrigin::Protocol { protocol } => {
                                MemberProjectionOrigin::Protocol {
                                    protocol: MemberProtocol {
                                        item: protocol.item,
                                        arguments: state.substitute_arguments(
                                            module,
                                            substitution,
                                            &protocol.arguments,
                                        )?,
                                    },
                                }
                            }
                        };
                        let member = MemberCallTerm {
                            origin,
                            receiver: member.receiver.substitute(module, substitution, state)?,
                            key: member.key,
                            arguments: state.substitute_arguments(
                                module,
                                substitution,
                                &member.arguments,
                            )?,
                        };

                        CallCallee::Member(state.inference.push_term(member))
                    }
                };
                let call = CallTerm {
                    source: call.source,
                    callee,
                    generic_arguments: state.substitute_arguments(
                        module,
                        substitution,
                        &call.generic_arguments,
                    )?,
                    arguments: call
                        .arguments
                        .iter()
                        .copied()
                        .map(|argument| argument.substitute(module, substitution, state))
                        .collect::<CompilerResult<_>>()?,
                };

                TypeTerm::Call(state.inference.push_term(call))
            }
            TypeTerm::Construct(construct) => {
                let construct = state.inference.term(*construct).clone();
                let construct = ConstructTerm {
                    source: construct.source,
                    callee: state.substitute_type_operand(
                        module,
                        substitution,
                        construct.callee,
                    )?,
                    generic_arguments: state.substitute_arguments(
                        module,
                        substitution,
                        &construct.generic_arguments,
                    )?,
                    arguments: construct
                        .arguments
                        .iter()
                        .copied()
                        .map(|argument| argument.substitute(module, substitution, state))
                        .collect::<CompilerResult<_>>()?,
                };
                let construct = state.inference.push_term(construct);

                TypeTerm::Construct(construct)
            }
            TypeTerm::RangeValue(range) => {
                let range = *state.inference.term(*range);
                let range = RangeTerm {
                    source: range.source,
                    start: range
                        .start
                        .map(|start| state.substitute_type_operand(module, substitution, start))
                        .transpose()?,
                    end: range
                        .end
                        .map(|end| state.substitute_type_operand(module, substitution, end))
                        .transpose()?,
                    end_kind: range.end_kind,
                };
                let range = state.inference.push_term(range);

                TypeTerm::RangeValue(range)
            }
            TypeTerm::Tree(tree) => {
                let tree = state.inference.term(*tree).clone();
                let tree = TreeTerm {
                    source: tree.source,
                    tag: tree
                        .tag
                        .map(|tag| state.substitute_type_operand(module, substitution, tag))
                        .transpose()?,
                    generic_arguments: state.substitute_arguments(
                        module,
                        substitution,
                        &tree.generic_arguments,
                    )?,
                    arguments: state.substitute_type_operands(
                        module,
                        substitution,
                        &tree.arguments,
                    )?,
                    elements: state.substitute_type_operands(
                        module,
                        substitution,
                        &tree.elements,
                    )?,
                };
                let tree = state.inference.push_term(tree);

                TypeTerm::Tree(tree)
            }
            TypeTerm::TypeValue(value) => {
                let value = *state.inference.term(*value);
                let value = TypeValueTerm {
                    source: value.source,
                    ty: state.substitute_type_operand(module, substitution, value.ty)?,
                };
                let value = state.inference.push_term(value);

                TypeTerm::TypeValue(value)
            }
            TypeTerm::ImportMeta(meta) => TypeTerm::ImportMeta(*meta),
            TypeTerm::Receiver(receiver) => {
                let receiver = *state.inference.term(*receiver);
                let receiver = ReceiverTerm {
                    source: receiver.source,
                    kind: receiver.kind,
                    owner: receiver.owner,
                    ty: state.substitute_type_operand(module, substitution, receiver.ty)?,
                };
                let receiver = state.inference.push_term(receiver);

                TypeTerm::Receiver(receiver)
            }
            TypeTerm::Super(term) => {
                let term = *state.inference.term(*term);
                let term = SuperTerm {
                    source: term.source,
                    receiver: term
                        .receiver
                        .map(|receiver| {
                            state.substitute_type_operand(module, substitution, receiver)
                        })
                        .transpose()?,
                };
                let term = state.inference.push_term(term);

                TypeTerm::Super(term)
            }
            TypeTerm::Operator(operator) => {
                let operator = *state.inference.term(*operator);
                let operator = OperatorTerm {
                    source: operator.source,
                    kind: operator.kind,
                    receiver: state.substitute_type_operand(
                        module,
                        substitution,
                        operator.receiver,
                    )?,
                    argument: operator
                        .argument
                        .map(|argument| {
                            state.substitute_type_operand(module, substitution, argument)
                        })
                        .transpose()?,
                };
                let operator = state.inference.push_term(operator);

                TypeTerm::Operator(operator)
            }
            TypeTerm::Index(index) => {
                let index = *state.inference.term(*index);
                let index = IndexTerm {
                    source: index.source,
                    kind: index.kind,
                    receiver: state.substitute_type_operand(
                        module,
                        substitution,
                        index.receiver,
                    )?,
                    index: state.substitute_type_operand(module, substitution, index.index)?,
                    key: index.key,
                };
                let index = state.inference.push_term(index);

                TypeTerm::Index(index)
            }
            TypeTerm::IndexSet(set) => {
                let set = *state.inference.term(*set);
                let set = IndexSetTerm {
                    source: set.source,
                    receiver: state.substitute_type_operand(module, substitution, set.receiver)?,
                    index: state.substitute_type_operand(module, substitution, set.index)?,
                    value: state.substitute_type_operand(module, substitution, set.value)?,
                    key: set.key,
                };
                let set = state.inference.push_term(set);

                TypeTerm::IndexSet(set)
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = *state.inference.term(*membership);
                let membership = KeyMembershipTerm {
                    source: membership.source,
                    key: state.substitute_type_operand(module, substitution, membership.key)?,
                    receiver: state.substitute_type_operand(
                        module,
                        substitution,
                        membership.receiver,
                    )?,
                };
                let membership = state.inference.push_term(membership);

                TypeTerm::KeyMembership(membership)
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = *state.inference.term(*instance);
                let instance = InstanceCheckTerm {
                    source: instance.source,
                    value: state.substitute_type_operand(module, substitution, instance.value)?,
                    target: state.substitute_type_operand(module, substitution, instance.target)?,
                };
                let instance = state.inference.push_term(instance);

                TypeTerm::InstanceCheck(instance)
            }
            TypeTerm::Identity(identity) => {
                let identity = *state.inference.term(*identity);
                let identity = IdentityTerm {
                    source: identity.source,
                    operator: identity.operator,
                    left: state.substitute_type_operand(module, substitution, identity.left)?,
                    right: state.substitute_type_operand(module, substitution, identity.right)?,
                };
                let identity = state.inference.push_term(identity);

                TypeTerm::Identity(identity)
            }
            TypeTerm::Await(awaited) => {
                let awaited = *state.inference.term(*awaited);
                let awaited = AwaitTerm {
                    source: awaited.source,
                    value: state.substitute_type_operand(module, substitution, awaited.value)?,
                };
                let awaited = state.inference.push_term(awaited);

                TypeTerm::Await(awaited)
            }
            TypeTerm::Try(tried) => {
                let tried = *state.inference.term(*tried);
                let tried = TryTerm {
                    source: tried.source,
                    value: state.substitute_type_operand(module, substitution, tried.value)?,
                    kind: tried.kind,
                };
                let tried = state.inference.push_term(tried);

                TypeTerm::Try(tried)
            }
            TypeTerm::Yield(yielded) => {
                let yielded = *state.inference.term(*yielded);
                let yielded = YieldTerm {
                    source: yielded.source,
                    value: yielded
                        .value
                        .map(|value| state.substitute_type_operand(module, substitution, value))
                        .transpose()?,
                    yield_target: yielded
                        .yield_target
                        .map(|ty| state.substitute_type_operand(module, substitution, ty))
                        .transpose()?,
                    resume_target: yielded
                        .resume_target
                        .map(|ty| state.substitute_type_operand(module, substitution, ty))
                        .transpose()?,
                    delegate_return_target: yielded
                        .delegate_return_target
                        .map(|ty| state.substitute_type_operand(module, substitution, ty))
                        .transpose()?,
                    cardinality: yielded.cardinality,
                };
                let yielded = state.inference.push_term(yielded);

                TypeTerm::Yield(yielded)
            }
            TypeTerm::TryFailure(tried) => {
                let tried = *state.inference.term(*tried);
                let tried = TryFailureTerm {
                    source: tried.source,
                    value: state.substitute_type_operand(module, substitution, tried.value)?,
                };
                let tried = state.inference.push_term(tried);

                TypeTerm::TryFailure(tried)
            }
            TypeTerm::Template(template) => {
                let template = state.inference.term(*template).clone();
                let template = TemplateTerm {
                    source: template.source,
                    strings: template.strings.clone(),
                    spans: state.substitute_type_operands(module, substitution, &template.spans)?,
                };
                let template = state.inference.push_term(template);

                TypeTerm::Template(template)
            }
            TypeTerm::TaggedTemplate(template) => {
                let template = state.inference.term(*template).clone();
                let template = TaggedTemplateTerm {
                    source: template.source,
                    tag: state.substitute_type_operand(module, substitution, template.tag)?,
                    generic_arguments: state.substitute_arguments(
                        module,
                        substitution,
                        &template.generic_arguments,
                    )?,
                    strings: template.strings.clone(),
                    spans: state.substitute_type_operands(module, substitution, &template.spans)?,
                };
                let template = state.inference.push_term(template);

                TypeTerm::TaggedTemplate(template)
            }
            TypeTerm::Dynamic { constraint } => TypeTerm::Dynamic {
                constraint: state.substitute_type_operand(module, substitution, *constraint)?,
            },
            TypeTerm::Closure {
                function,
                environment,
            } => TypeTerm::Closure {
                function: state.substitute_type_operand(module, substitution, *function)?,
                environment: state.substitute_type_operand(module, substitution, *environment)?,
            },
            TypeTerm::This => return Ok(None),
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::Type(_)
            | TypeTerm::Range { .. } => return Ok(None),
        };

        Ok(Some(state.type_term_operand(term)))
    }
}

impl CheckState<'_> {
    /// Decide exact equality for type variable lists.
    pub(in crate::check) fn decide_type_variable_list_equal<L, R>(
        &mut self,
        left: &[L],
        right: &[R],
    ) -> CompilerResult<Answer<bool>>
    where
        L: Copy + Into<TypeOperand>,
        R: Copy + Into<TypeOperand>,
    {
        if left.len() != right.len() {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // compare matching variables in declaration order
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Equal,
                (*left).into(),
                (*right).into(),
            )?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for optional type operands.
    pub(in crate::check) fn decide_optional_type_operand_equal(
        &mut self,
        left: Option<TypeOperand>,
        right: Option<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (left, right) {
            (Some(left), Some(right)) => {
                self.decide_type_relation(TypeRelation::Equal, left, right)?
            }
            (None, None) => Answer::Ready(true),
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Check one operand as a computed definition for its result variable.
    pub(in crate::check) fn expect_type_operand_bound(
        &mut self,
        origin: Origin,
        result: VariableId,
        operand: TypeOperand,
    ) -> CompilerResult<Option<Answer<()>>> {
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(None);
        };

        let answer = self.expect_type_bound_term(origin, result, term)?;

        Ok(answer)
    }

    /// Check one term as a computed definition for its result variable.
    pub(in crate::check) fn expect_type_bound_term(
        &mut self,
        origin: Origin,
        result: VariableId,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Option<Answer<()>>> {
        let handled = match self.inference.term(term) {
            TypeTerm::Call(call) => Some(self.expect_call_term(origin, *call, result)?),
            TypeTerm::Construct(construct) => {
                Some(self.expect_construct_term(origin, *construct, result)?)
            }
            TypeTerm::Operator(operator) => {
                let operator = *self.inference.term(*operator);

                Some(self.expect_operator_term(origin, &operator, result, None)?)
            }
            TypeTerm::Index(index) => {
                let index = *self.inference.term(*index);

                Some(self.expect_index_term(origin, &index, result)?)
            }
            TypeTerm::IndexSet(set) => {
                let set = *self.inference.term(*set);

                Some(self.expect_index_set_term(origin, &set, result)?)
            }
            TypeTerm::TaggedTemplate(template) => {
                Some(self.expect_tagged_template_term(origin, *template, result)?)
            }
            TypeTerm::Await(awaited) => {
                let awaited = *self.inference.term(*awaited);

                Some(self.expect_await_term(origin, &awaited, result)?)
            }
            TypeTerm::Try(tried) => {
                let tried = *self.inference.term(*tried);

                Some(self.expect_try_term(origin, &tried, result)?)
            }
            TypeTerm::Yield(yielded) => {
                let yielded = *self.inference.term(*yielded);

                Some(self.expect_yield_term(origin, &yielded, result)?)
            }
            TypeTerm::TryFailure(tried) => {
                let tried = *self.inference.term(*tried);

                Some(self.expect_try_failure_term(origin, &tried, result)?)
            }
            TypeTerm::Operation(operation) => match self.inference.term(*operation) {
                TypeOperationTerm::Exclude { source, target } => {
                    let source = *source;
                    let target = *target;
                    let term = self.reduce_exclude_term(origin, source, target)?;
                    let Answer::Ready(term) = term else {
                        return Ok(Some(term.map(|_| ())));
                    };
                    self.constrain_type(
                        origin,
                        TypeRelation::Equal,
                        result,
                        term,
                        Condition::Always,
                    );

                    Some(Answer::Ready(()))
                }
                TypeOperationTerm::Extract { source, target } => {
                    let source = *source;
                    let target = *target;
                    let term = self.reduce_extract_term(origin, source, target)?;
                    let Answer::Ready(term) = term else {
                        return Ok(Some(term.map(|_| ())));
                    };
                    self.constrain_type(
                        origin,
                        TypeRelation::Equal,
                        result,
                        term,
                        Condition::Always,
                    );

                    Some(Answer::Ready(()))
                }
                TypeOperationTerm::Widen { source } => {
                    let source = *source;

                    self.constrain_type(
                        origin,
                        TypeRelation::Assignable,
                        source,
                        result,
                        Condition::Always,
                    );

                    Some(Answer::Ready(()))
                }
                TypeOperationTerm::Conditional {
                    left,
                    right,
                    then_type,
                    else_type,
                } => Some(self.expect_conditional_term(
                    origin,
                    *left,
                    *right,
                    *then_type,
                    *else_type,
                    result.into(),
                )?),
                TypeOperationTerm::Index { left, index } => Some(self.expect_type_index_term(
                    origin,
                    result.module,
                    *left,
                    *index,
                    result.into(),
                )?),
                TypeOperationTerm::BestCommon { .. }
                | TypeOperationTerm::TemplateLiteral { .. }
                | TypeOperationTerm::Infer { .. }
                | TypeOperationTerm::KeyOf { .. }
                | TypeOperationTerm::Mapped { .. }
                | TypeOperationTerm::StringMapping { .. }
                | TypeOperationTerm::Intrinsic { .. } => None,
            },
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::Type(_)
            | TypeTerm::Form { .. }
            | TypeTerm::Reference { .. }
            | TypeTerm::This
            | TypeTerm::Member(_)
            | TypeTerm::Parameter(_)
            | TypeTerm::StaticValue { .. }
            | TypeTerm::Array { .. }
            | TypeTerm::Tuple { .. }
            | TypeTerm::Shape(_)
            | TypeTerm::FixedArray { .. }
            | TypeTerm::Slice { .. }
            | TypeTerm::Union { .. }
            | TypeTerm::Intersection { .. }
            | TypeTerm::Function(_)
            | TypeTerm::Range { .. }
            | TypeTerm::RangeValue(_)
            | TypeTerm::Tree(_)
            | TypeTerm::TypeValue(_)
            | TypeTerm::ImportMeta(_)
            | TypeTerm::Receiver(_)
            | TypeTerm::Super(_)
            | TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_)
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Closure { .. } => None,
        };

        Ok(handled)
    }

    /// Return the contextual view of one type operand.
    pub(in crate::check) fn contextual_type_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let mut seen = Vec::new();

        self.contextual_type_operand_inner(origin, operand, &mut seen)
    }

    /// Return the contextual view of one operand while tracking aliases.
    fn contextual_type_operand_inner(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
        seen: &mut Vec<dir::GlobalSymbolId>,
    ) -> CompilerResult<TypeOperand> {
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(operand);
        };
        let TypeTerm::Reference {
            origin: _,
            symbol,
            arguments,
        } = self.inference.term(term)
        else {
            return Ok(term.into());
        };
        let symbol = *symbol;
        let arguments = arguments
            .iter()
            .copied()
            .collect::<SmallVec<[GenericArgument; 2]>>();
        if seen.contains(&symbol) {
            return Err(self.circular_type_error(origin)?.into());
        }

        seen.push(symbol);
        let Some(operand) = self.contextual_reference_operand(origin, symbol, &arguments, seen)?
        else {
            seen.pop();

            return Ok(term.into());
        };
        seen.pop();

        Ok(operand)
    }

    /// Return the substituted operand of one transparent type alias.
    fn contextual_reference_operand(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        seen: &mut Vec<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let value = {
            let Some(definition) = self.definitions.definition(symbol) else {
                return Ok(None);
            };
            let Definition::TypeAlias(definition) = definition else {
                return Ok(None);
            };

            definition.value
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        let value = if substitution.is_empty() {
            value
        } else {
            self.substitute_type_operand(origin.module(), &substitution, value)?
        };
        let value = self.contextual_type_operand_inner(origin, value, seen)?;

        Ok(Some(value))
    }

    /// Decide whether an explicit cast is valid.
    fn decide_type_castable(
        &mut self,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let forward = self.decide_type_assignable(source, target)?;
        if forward == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }
        let backward = self.decide_type_assignable(target, source)?;
        if backward == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        Ok(forward.or(backward))
    }

    /// Decide exact type equality.
    fn decide_type_equal(
        &mut self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        if left == right {
            return Ok(Answer::Ready(true));
        }

        let decision = match (self.inference.term(left), self.inference.term(right)) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), right) if right.is_unit() => {
                Answer::Ready(true)
            }
            (left, TypeTerm::Literal(TypeLiteralTerm::Void)) if left.is_unit() => {
                Answer::Ready(true)
            }
            (
                TypeTerm::Form {
                    form: left_form,
                    payload: left_value,
                },
                TypeTerm::Form {
                    form: right_form,
                    payload: right_value,
                },
            ) => {
                let form = self.decide_form_equal(
                    self.inference.term(*left_form),
                    self.inference.term(*right_form),
                )?;
                if form != Answer::Ready(true) {
                    return Ok(form);
                }

                self.decide_type_relation(TypeRelation::Equal, *left_value, *right_value)?
            }
            (TypeTerm::Literal(left), TypeTerm::Literal(right)) => {
                self.decide_type_literal_equal(left, right)
            }
            (
                TypeTerm::Reference {
                    origin: _,
                    symbol: left_symbol,
                    arguments: _,
                },
                TypeTerm::Reference {
                    origin: _,
                    symbol: right_symbol,
                    arguments: _,
                },
            ) => {
                if left_symbol != right_symbol {
                    Answer::Ready(false)
                } else {
                    self.decide_reference_arguments_equal(left, right)?
                }
            }
            (TypeTerm::Array { element: left }, TypeTerm::Array { element: right }) => {
                let left = *left;
                let right = *right;

                self.decide_type_relation(TypeRelation::Equal, left, right)?
            }
            (
                TypeTerm::Slice {
                    element: left_element,
                },
                TypeTerm::Slice {
                    element: right_element,
                },
            ) => {
                let left_element = *left_element;
                let right_element = *right_element;

                self.decide_type_relation(TypeRelation::Equal, left_element, right_element)?
            }
            (
                TypeTerm::FixedArray {
                    element: left_element,
                    length: left_length,
                },
                TypeTerm::FixedArray {
                    element: right_element,
                    length: right_length,
                },
            ) => {
                let left_element = *left_element;
                let right_element = *right_element;
                let left_length = *left_length;
                let right_length = *right_length;
                let element =
                    self.decide_type_relation(TypeRelation::Equal, left_element, right_element)?;
                let length =
                    self.decide_static_relation(StaticRelation::Equal, left_length, right_length)?;

                element.and(length)
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: _,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: _,
                },
            ) => {
                if left_form != right_form {
                    Answer::Ready(false)
                } else {
                    self.decide_tuple_terms_equal(left, right)?
                }
            }
            (TypeTerm::Shape(left), TypeTerm::Shape(right)) => {
                self.decide_shape_terms_equal(*left, *right)?
            }
            (TypeTerm::Function(left), TypeTerm::Function(right)) => {
                self.decide_function_equal(*left, *right)?
            }
            (
                TypeTerm::Range {
                    start: left_start,
                    end: left_end,
                    is_inclusive: left_inclusive,
                },
                TypeTerm::Range {
                    start: right_start,
                    end: right_end,
                    is_inclusive: right_inclusive,
                },
            ) => {
                if left_start == right_start
                    && left_end == right_end
                    && left_inclusive == right_inclusive
                {
                    Answer::Ready(true)
                } else {
                    Answer::Ready(false)
                }
            }
            (TypeTerm::Union { elements: left }, TypeTerm::Union { elements: right })
            | (
                TypeTerm::Intersection { elements: left },
                TypeTerm::Intersection { elements: right },
            ) => {
                let left = left.clone();
                let right = right.clone();

                self.decide_type_variable_list_equal(&left, &right)?
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide assignability from source to target.
    fn decide_type_assignable(
        &mut self,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_type_equal(source, target)? == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        let decision = match (self.inference.term(source), self.inference.term(target)) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), target) if target.is_unit() => {
                Answer::Ready(true)
            }
            (source, TypeTerm::Literal(TypeLiteralTerm::Void)) if source.is_unit() => {
                Answer::Ready(true)
            }
            (
                TypeTerm::Form {
                    form: source_form,
                    payload: source_value,
                },
                TypeTerm::Form {
                    form: target_form,
                    payload: target_value,
                },
            ) => {
                let source_form = *source_form;
                let target_form = *target_form;
                let source_value = *source_value;
                let target_value = *target_value;
                let form = self.decide_form_assignable(
                    self.inference.term(source_form),
                    self.inference.term(target_form),
                )?;
                if form != Answer::Ready(true) {
                    return Ok(form);
                }

                self.decide_type_relation(TypeRelation::Assignable, source_value, target_value)?
            }
            (TypeTerm::Union { .. }, _) => {
                self.decide_all_union_sources_assignable(source, target)?
            }
            (_, TypeTerm::Union { .. }) => {
                self.decide_any_union_target_assignable(source, target)?
            }
            (TypeTerm::Literal(source), TypeTerm::Literal(target)) => {
                self.decide_type_atom_assignable(source, target)
            }
            (TypeTerm::Array { element: source }, TypeTerm::Array { element: target }) => {
                let source = *source;
                let target = *target;

                self.decide_type_relation(TypeRelation::Assignable, source, target)?
            }
            (TypeTerm::Array { element: source }, TypeTerm::Slice { element: target }) => {
                let source = *source;
                let target = *target;

                self.decide_type_relation(TypeRelation::Assignable, source, target)?
            }
            (TypeTerm::Slice { element: source }, TypeTerm::Slice { element: target }) => {
                let source = *source;
                let target = *target;

                self.decide_type_relation(TypeRelation::Assignable, source, target)?
            }
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: source_length,
                },
                TypeTerm::FixedArray {
                    element: target_element,
                    length: target_length,
                },
            ) => {
                let source_element = *source_element;
                let target_element = *target_element;
                let source_length = *source_length;
                let target_length = *target_length;
                let element = self.decide_type_relation(
                    TypeRelation::Assignable,
                    source_element,
                    target_element,
                )?;
                let length = self.decide_static_relation(
                    StaticRelation::Equal,
                    source_length,
                    target_length,
                )?;

                element.and(length)
            }
            (
                TypeTerm::FixedArray {
                    element: source, ..
                },
                TypeTerm::Slice { element: target },
            ) => {
                let source = *source;
                let target = *target;

                self.decide_type_relation(TypeRelation::Assignable, source, target)?
            }
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: _,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: _,
                },
            ) => {
                if source_form != target_form {
                    Answer::Ready(false)
                } else {
                    self.decide_tuple_terms_assignable(source, target)?
                }
            }
            (TypeTerm::Shape(source), TypeTerm::Shape(target)) => {
                self.decide_shape_terms_assignable(*source, *target)?
            }
            (
                TypeTerm::Shape(source),
                TypeTerm::Reference {
                    origin: _,
                    symbol,
                    arguments,
                },
            ) => {
                let source = self
                    .inference
                    .term(*source)
                    .members
                    .iter()
                    .copied()
                    .collect::<SmallVec<[ShapeMember; 4]>>();

                let symbol = *symbol;
                let arguments = arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();

                self.decide_shape_assignable_to_interface(&source, symbol, &arguments)?
            }
            (
                TypeTerm::Reference {
                    origin: _,
                    symbol,
                    arguments,
                },
                TypeTerm::Shape(target),
            ) => {
                let target = self
                    .inference
                    .term(*target)
                    .members
                    .iter()
                    .copied()
                    .collect::<SmallVec<[ShapeMember; 4]>>();

                let symbol = *symbol;
                let arguments = arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();

                self.decide_reference_assignable_to_shape(symbol, &arguments, &target)?
            }
            (TypeTerm::Parameter(parameter), _) => {
                let parameter = *parameter;

                self.decide_type_parameter_assignable(parameter, target)?
            }
            (
                TypeTerm::Reference {
                    origin: _,
                    symbol: source_symbol,
                    arguments: source_arguments,
                },
                TypeTerm::Reference {
                    origin: _,
                    symbol: target_symbol,
                    arguments: target_arguments,
                },
            ) if source_symbol == target_symbol => {
                let source_symbol = *source_symbol;

                self.decide_reference_term_arguments_assignable(source_symbol, source, target)?
            }
            (source, TypeTerm::Reference { symbol, .. })
                if !source.can_be_nominal() && !self.symbol_kind(*symbol).is_interface() =>
            {
                Answer::Ready(false)
            }
            (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                self.decide_function_assignable(*source, *target)?
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether one type parameter satisfies one target through its constraint.
    fn decide_type_parameter_assignable(
        &mut self,
        parameter: GenericParameterId,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(Answer::Ready(false));
        };
        let Some(origin) = self.generic_parameter_origin(parameter) else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} has no source origin"),
            });
        };
        let Some(constraint) = self.reduce_generic_constraint(origin, constraint)? else {
            let Some(parameter) = self.inference.generic_parameter(parameter) else {
                return Err(CompilerError::Internal {
                    message: format!("generic parameter {parameter:?} is not allocated"),
                });
            };
            let Some(constraint) = parameter.type_constraint() else {
                return Ok(Answer::Ready(false));
            };

            return Ok(Answer::pending(constraint.dependencies(self)));
        };
        self.decide_type_relation(TypeRelation::Assignable, constraint, target)
    }

    /// Decide same-symbol reference argument assignability.
    fn decide_reference_term_arguments_assignable(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let Some((source_len, target_len)) = self.reference_argument_lengths(source, target) else {
            return Ok(Answer::Ready(false));
        };
        if source_len != target_len {
            return Ok(Answer::Ready(false));
        }
        if source_len == 0 {
            return Ok(Answer::Ready(true));
        }
        self.definitions.definition(symbol);
        let Some(template) = self.inference.generic_template_by_symbol(symbol) else {
            return Ok(Answer::Ready(false));
        };
        let Some(parameter_len) = self
            .inference
            .generic_template(template)
            .map(|template| template.parameters.len())
        else {
            return Ok(Answer::Ready(false));
        };
        if parameter_len != source_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // compare each argument by its declared parameter variance
        for index in 0..source_len {
            let Some(parameter) = self
                .inference
                .generic_template(template)
                .and_then(|template| template.parameters.get(index))
                .copied()
            else {
                return Ok(Answer::Ready(false));
            };
            let parameter = *self.inference.generic_parameter_binding(parameter)?;
            let Some((source, target)) = self.reference_argument_pair(source, target, index) else {
                return Ok(Answer::Ready(false));
            };
            let argument =
                self.decide_reference_argument_assignable(&parameter, &source, &target)?;

            decision = decision.and(argument);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for same-symbol reference arguments.
    fn decide_reference_arguments_equal(
        &mut self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let Some((left_len, right_len)) = self.reference_argument_lengths(left, right) else {
            return Ok(Answer::Ready(false));
        };
        if left_len != right_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // compare arguments in declaration order
        for index in 0..left_len {
            let Some((left, right)) = self.reference_argument_pair(left, right, index) else {
                return Ok(Answer::Ready(false));
            };

            decision = decision.and(self.decide_argument_equal(&left, &right)?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Constrain exact equality for same-symbol reference arguments.
    fn constrain_reference_arguments_equal(
        &mut self,
        origin: Origin,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<()> {
        let Some((left_len, right_len)) = self.reference_argument_lengths(left, right) else {
            return Ok(());
        };
        if left_len != right_len {
            return Ok(());
        }

        // constrain arguments in declaration order
        for index in 0..left_len {
            let Some((left, right)) = self.reference_argument_pair(left, right, index) else {
                return Ok(());
            };

            self.constrain_argument_equal(origin, &left, &right)?;
        }

        Ok(())
    }

    /// Return reference argument lengths for two reference terms.
    fn reference_argument_lengths(
        &self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> Option<(usize, usize)> {
        match (self.inference.term(left), self.inference.term(right)) {
            (
                TypeTerm::Reference {
                    arguments: left, ..
                },
                TypeTerm::Reference {
                    arguments: right, ..
                },
            ) => Some((left.len(), right.len())),
            _ => None,
        }
    }

    /// Return one cloned argument pair from two reference terms.
    fn reference_argument_pair(
        &self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
        index: usize,
    ) -> Option<(GenericArgument, GenericArgument)> {
        match (self.inference.term(left), self.inference.term(right)) {
            (
                TypeTerm::Reference {
                    arguments: left, ..
                },
                TypeTerm::Reference {
                    arguments: right, ..
                },
            ) => Some((left.get(index)?.clone(), right.get(index)?.clone())),
            _ => None,
        }
    }

    /// Decide one same-symbol reference argument by parameter variance.
    fn decide_reference_argument_assignable(
        &mut self,
        parameter: &GenericParameterBinding,
        source: &GenericArgument,
        target: &GenericArgument,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (source.type_operand(), target.type_operand()) {
            (Some(source), Some(target)) => match parameter.variance() {
                Some(dir::VarianceModifier::Out) => {
                    self.decide_type_relation(TypeRelation::Assignable, source, target)?
                }
                Some(dir::VarianceModifier::In) => {
                    self.decide_type_relation(TypeRelation::Assignable, target, source)?
                }
                Some(dir::VarianceModifier::InOut) | None => {
                    self.decide_type_relation(TypeRelation::Equal, source, target)?
                }
            },
            (None, None) => self.decide_argument_equal(source, target)?,
            (Some(_), None) | (None, Some(_)) => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether source satisfies one target constraint.
    fn decide_type_satisfies(
        &mut self,
        module: Option<ModuleId>,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_type_equal(source, target)? == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        let decision = match (self.inference.term(source), self.inference.term(target)) {
            (TypeTerm::Shape(source), TypeTerm::Shape(target)) => {
                self.decide_shape_terms_satisfies(*source, *target)?
            }
            (
                TypeTerm::Reference {
                    origin: _,
                    symbol: source_symbol,
                    arguments: source_arguments,
                },
                TypeTerm::Reference {
                    origin: _,
                    symbol: target_symbol,
                    arguments: target_arguments,
                },
            ) => {
                let source_symbol = *source_symbol;
                let target_symbol = *target_symbol;
                let source_arguments = source_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();
                let target_arguments = target_arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();

                self.decide_nominal_satisfies(
                    module,
                    source_symbol,
                    &source_arguments,
                    target_symbol,
                    &target_arguments,
                )?
            }
            _ => self.decide_type_assignable(source, target)?,
        };

        Ok(decision)
    }

    /// Decide whether one nominal reference satisfies another nominal constraint.
    fn decide_nominal_satisfies(
        &mut self,
        module: Option<ModuleId>,
        source_symbol: dir::GlobalSymbolId,
        source_arguments: &[GenericArgument],
        target_symbol: dir::GlobalSymbolId,
        target_arguments: &[GenericArgument],
    ) -> CompilerResult<Answer<bool>> {
        if !self.symbol_kind(target_symbol).is_interface() {
            return Ok(Answer::Ready(false));
        }
        let Some(target_definition) = self.definitions.definition(target_symbol) else {
            return Err(CompilerError::Internal {
                message: format!("target symbol {target_symbol:?} has no definition"),
            });
        };
        let target_members = target_definition.named_type_members();
        let target_substitution = self.generic_substitution(target_symbol, target_arguments)?;
        let source = TypeTerm::Reference {
            origin: Origin::Symbol(source_symbol),
            symbol: source_symbol,
            arguments: source_arguments.to_vec().into(),
        };
        let source_operand = self.type_term_operand(source.clone());
        let mut decision = Answer::Ready(true);
        let module = module.unwrap_or(source_symbol.module_id);

        // require every interface member from the source nominal type
        for target_member in target_members {
            let key = target_member.key();
            let Some(target_type) = target_member.value() else {
                let member = self.decide_nominal_associated_type_satisfies(
                    module,
                    source_operand,
                    key,
                    target_symbol,
                )?;

                decision = decision.and(member);
                if decision == Answer::Ready(false) {
                    return Ok(decision);
                }

                continue;
            };
            let member = self.decide_nominal_member_satisfies(
                module,
                source_operand,
                key,
                target_symbol,
                target_type,
                &target_substitution,
            )?;

            decision = decision.and(member);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source nominal type has a required associated type.
    fn decide_nominal_associated_type_satisfies(
        &mut self,
        module: ModuleId,
        source: TypeOperand,
        key: dir::StaticKey,
        target_owner: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        let receiver = MemberReceiver::Value(source.into());
        let candidates =
            self.resolve_member(Origin::Symbol(target_owner), module, &receiver, &key)?;

        match candidates {
            MemberLookup::Field(_) | MemberLookup::Found(_) => Ok(Answer::Ready(true)),
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
            MemberLookup::Missing => Ok(Answer::Ready(false)),
        }
    }

    /// Decide whether one source nominal member satisfies one target member.
    fn decide_nominal_member_satisfies(
        &mut self,
        module: ModuleId,
        source: TypeOperand,
        key: dir::StaticKey,
        target_owner: dir::GlobalSymbolId,
        target_member: TypeOperand,
        target_substitution: &SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let receiver = MemberReceiver::Value(source.into());
        let candidates =
            self.resolve_member(Origin::Symbol(target_owner), module, &receiver, &key)?;
        let target_member = if target_substitution.is_empty() {
            target_member
        } else {
            self.substitute_type_operand(
                target_owner.module_id,
                target_substitution,
                target_member,
            )?
        };

        match candidates {
            MemberLookup::Field(member) => {
                self.decide_member_type_satisfies(module, member, target_member)
            }
            MemberLookup::Found(candidates) => {
                let mut decision = Answer::Ready(false);

                for candidate in candidates {
                    let Some(member) = candidate.ty else {
                        continue;
                    };

                    let candidate =
                        self.decide_member_type_satisfies(module, member, target_member)?;
                    decision = decision.or(candidate);
                    if decision == Answer::Ready(true) {
                        return Ok(decision);
                    }
                }

                Ok(decision)
            }
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
            MemberLookup::Missing => Ok(Answer::Ready(false)),
        }
    }

    /// Decide whether one member type satisfies another member type.
    fn decide_member_type_satisfies(
        &mut self,
        module: ModuleId,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Answer<bool>> {
        let Some(source_term) = self.type_operand_term_id(source)? else {
            return Ok(Answer::pending(source.dependencies(self)));
        };
        let Some(target_term) = self.type_operand_term_id(target)? else {
            return Ok(Answer::pending(target.dependencies(self)));
        };

        match (
            self.inference.term(source_term),
            self.inference.term(target_term),
        ) {
            (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                self.decide_member_function_satisfies(*source, *target)
            }
            _ => self.decide_type_term_id_relation(
                Some(module),
                TypeRelation::Satisfies,
                source_term,
                target_term,
            ),
        }
    }

    /// Decide whether one member function satisfies another member function.
    fn decide_member_function_satisfies(
        &mut self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let source_term = self.inference.term(source);
        let source_asynchrony = source_term.asynchrony;
        let source_is_generator = source_term.is_generator;

        let target_term = self.inference.term(target);
        let target_asynchrony = target_term.asynchrony;
        let target_is_generator = target_term.is_generator;

        if source_asynchrony != target_asynchrony || source_is_generator != target_is_generator {
            return Ok(Answer::Ready(false));
        }

        self.decide_member_function_signature_satisfies(source, target)
    }

    /// Decide whether one member function signature satisfies another.
    fn decide_member_function_signature_satisfies(
        &mut self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let source_term = self.inference.term(source);
        let source_len = source_term.parameters.len();
        let source_return = source_term.return_type;

        let target_term = self.inference.term(target);
        let target_len = target_term.parameters.len();
        let target_return = target_term.return_type;

        if source_len != target_len {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // compare runtime parameters contravariantly
        for index in 0..source_len {
            let source = self.inference.term(source).parameters[index];
            let target = self.inference.term(target).parameters[index];

            if source.is_optional != target.is_optional || source.is_rest != target.is_rest {
                return Ok(Answer::Ready(false));
            }

            decision = decision.and(self.decide_type_relation(
                TypeRelation::Satisfies,
                target.ty,
                source.ty,
            )?);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        // compare return types covariantly
        match (source_return, target_return) {
            (Some(source), Some(target)) => {
                decision = decision.and(self.decide_type_relation(
                    TypeRelation::Satisfies,
                    source,
                    target,
                )?);
            }
            (None, None) => {}
            _ => return Ok(Answer::Ready(false)),
        }

        Ok(decision)
    }

    /// Reduce one type declaration reference.
    fn reduce_reference_term(
        &mut self,
        origin: Origin,
        term: TermId<TypeTerm>,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Answer<TypeOperand>> {
        if let Some(value) = self.type_alias_reference_value(origin, symbol, arguments)? {
            return Ok(Answer::Ready(value));
        }

        Ok(Answer::Ready(TypeOperand::Term(term)))
    }

    /// Return the substituted value of one transparent type alias reference.
    fn type_alias_reference_value(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeOperand>> {
        let value = {
            let Some(definition) = self.definitions.definition(symbol) else {
                return Ok(None);
            };
            let Definition::TypeAlias(definition) = definition else {
                return Ok(None);
            };

            definition.value
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        let value = if substitution.is_empty() {
            value
        } else {
            self.substitute_type_operand(origin.module(), &substitution, value)?
        };

        Ok(Some(value))
    }

    /// Reduce one static value used as a type.
    pub(in crate::check) fn reduce_static_value_term(
        &mut self,
        module: ModuleId,
        value: StaticOperand,
    ) -> CompilerResult<Answer<TypeTerm>> {
        let (value_module, value, origin) = match value {
            StaticOperand::Variable(variable) => {
                let Some(operand) = self.resolved_static_variable(variable) else {
                    return Ok(Answer::pending([Dependency::Variable(variable)]));
                };

                (
                    variable.module,
                    operand,
                    Some(self.variable(variable).source),
                )
            }
            StaticOperand::Term(_) => (module, value, None),
            StaticOperand::Static(value) => (value.module_id, value.into(), None),
        };
        let value = if let Some(origin) = origin {
            let Answer::Ready(value) = self.reduce_static_operand(origin, value)? else {
                return Ok(Answer::pending(value.dependencies(self)));
            };

            value
        } else {
            value
        };
        let Some(term) = self.type_from_static_operand(module, value_module, value)? else {
            return Ok(Answer::pending(value.dependencies(self)));
        };

        Ok(Answer::Ready(term))
    }

    /// Return the singleton type described by one static operand.
    fn type_from_static_operand(
        &mut self,
        module: ModuleId,
        value_module: ModuleId,
        operand: StaticOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        match operand {
            StaticOperand::Term(term) => self.type_from_static_term_id(module, value_module, term),
            StaticOperand::Static(value) => {
                let term = self.r#static(value).clone();

                self.type_from_static_value(module, value.module_id, term)
            }
            StaticOperand::Variable(_) => return Ok(None),
        }
    }

    /// Return the singleton type described by one stored static term.
    fn type_from_static_term_id(
        &mut self,
        module: ModuleId,
        value_module: ModuleId,
        term: TermId<StaticTerm>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match self.inference.term(term) {
            &StaticTerm::Static(value) => {
                let term = self.r#static(value).clone();

                return self.type_from_static_value(module, value.module_id, term);
            }
            &StaticTerm::Expression(expression) => {
                let Some(term) = self.static_expression_term(expression)? else {
                    return Ok(None);
                };
                let origin = Origin::Node(expression.into_any());
                let term = self.inference.push_term(term);
                let Answer::Ready(term) = self.reduce_static_term(origin, term)? else {
                    return Ok(None);
                };

                return self.type_from_static_operand(module, expression.module_id, term);
            }
            StaticTerm::Literal(value) => {
                return self.type_from_static_value(module, value_module, value.clone());
            }
            &StaticTerm::Parameter(parameter) => TypeTerm::Parameter(parameter),
            StaticTerm::Member { .. }
            | StaticTerm::Union { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Return the singleton type described by one owned static value.
    fn type_from_static_value(
        &mut self,
        module: ModuleId,
        value_module: ModuleId,
        value: dir::StaticTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        self.type_from_static_value_borrowed(module, value_module, &value)
    }

    /// Return the singleton type described by one borrowed static value.
    fn type_from_static_value_borrowed(
        &mut self,
        module: ModuleId,
        value_module: ModuleId,
        value: &dir::StaticTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match value {
            dir::StaticTerm::ScalarLiteral { value } => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(*value))
            }
            dir::StaticTerm::Type { ty } => TypeTerm::Type(*ty),
            dir::StaticTerm::TypeLiteral { value } => {
                let ty = dir::Type::from(value.clone());
                let Some(literal) = TypeLiteralTerm::from_type(&ty) else {
                    return Ok(None);
                };

                TypeTerm::Literal(literal)
            }
            dir::StaticTerm::Parameter(parameter) => TypeTerm::Parameter(*parameter),
            dir::StaticTerm::Object { properties } => {
                let mut members = Vec::with_capacity(properties.len());
                for property in properties {
                    let dir::StaticProperty::Field { key, value } = property else {
                        return Ok(None);
                    };
                    let Some(term) =
                        self.type_from_static_value_borrowed(module, value_module, value)?
                    else {
                        return Ok(None);
                    };
                    let ty = self.inference.push_term(term);

                    let member = ShapeMember::Field {
                        key: *key,
                        ty: ty.into(),
                        is_optional: false,
                        is_readonly: false,
                    };

                    members.push(member);
                }

                self.push_shape_type(members.into())
            }
            _ => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Decide exact atom equality.
    fn decide_type_literal_equal(
        &self,
        left: &TypeLiteralTerm,
        right: &TypeLiteralTerm,
    ) -> Answer<bool> {
        if left == right {
            Answer::Ready(true)
        } else {
            Answer::Ready(false)
        }
    }

    /// Decide atom assignability.
    fn decide_type_atom_assignable(
        &self,
        source: &TypeLiteralTerm,
        target: &TypeLiteralTerm,
    ) -> Answer<bool> {
        if source == target {
            return Answer::Ready(true);
        }

        match (source, target) {
            (TypeLiteralTerm::Error, _) | (_, TypeLiteralTerm::Error) => Answer::Ready(true),
            (TypeLiteralTerm::Any, _) | (_, TypeLiteralTerm::Any) => Answer::Ready(true),
            (TypeLiteralTerm::Never, _) => Answer::Ready(true),
            (_, TypeLiteralTerm::Unknown) => Answer::Ready(true),
            (TypeLiteralTerm::Unknown, _) => Answer::Ready(false),
            (TypeLiteralTerm::Scalar(literal), target) => {
                self.decide_literal_assignable(literal, target)
            }
            _ => Answer::Ready(false),
        }
    }

    /// Decide literal widening assignability.
    fn decide_literal_assignable(
        &self,
        literal: &dir::ScalarLiteral,
        target: &TypeLiteralTerm,
    ) -> Answer<bool> {
        match (literal, target) {
            (dir::ScalarLiteral::Null, TypeLiteralTerm::Null) => Answer::Ready(true),
            (dir::ScalarLiteral::Undefined, TypeLiteralTerm::Undefined) => Answer::Ready(true),
            (dir::ScalarLiteral::Boolean(_), target) if target == &TypeLiteralTerm::boolean() => {
                Answer::Ready(true)
            }
            (
                dir::ScalarLiteral::Character(_),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Character),
            ) => Answer::Ready(true),
            (
                dir::ScalarLiteral::String(_),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::String),
            ) => Answer::Ready(true),
            (dir::ScalarLiteral::Bigint(_), target) if target == &TypeLiteralTerm::bigint() => {
                Answer::Ready(true)
            }
            (
                dir::ScalarLiteral::Integer(value),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Integer(integer)),
            ) if Self::integer_literal_fits_integer(*value, *integer) => Answer::Ready(true),
            (
                dir::ScalarLiteral::Integer(value),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Float(float)),
            ) if Self::integer_literal_fits_float(*value, *float) => Answer::Ready(true),
            (
                dir::ScalarLiteral::Float(value),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Float(float)),
            ) if Self::float_literal_fits_float(*value, *float) => Answer::Ready(true),
            (dir::ScalarLiteral::RegexString { .. }, TypeLiteralTerm::Object) => {
                Answer::Ready(true)
            }
            _ => Answer::Ready(false),
        }
    }

    /// Return whether an integer literal fits one integer primitive.
    fn integer_literal_fits_integer(value: i64, integer: dir::IntegerType) -> bool {
        let value = i128::from(value);

        match integer {
            dir::IntegerType::Integer { is_signed } | dir::IntegerType::Pointer { is_signed } => {
                is_signed || value >= 0
            }
            dir::IntegerType::Fixed { width, is_signed } => {
                if width == 0 {
                    return false;
                }
                if is_signed {
                    let limit = 1_i128.checked_shl(u32::from(width - 1));
                    let Some(limit) = limit else {
                        return true;
                    };

                    value >= -limit && value < limit
                } else {
                    let limit = 1_i128.checked_shl(u32::from(width));
                    let Some(limit) = limit else {
                        return value >= 0;
                    };

                    value >= 0 && value < limit
                }
            }
        }
    }

    /// Return whether an integer literal fits one float primitive exactly.
    fn integer_literal_fits_float(value: i64, float: dir::FloatType) -> bool {
        let value = value as f64;
        float
            .roundtrip_f64(value)
            .is_some_and(|rounded| rounded == value)
    }

    /// Return whether a float literal fits one float primitive exactly.
    fn float_literal_fits_float(value: f64, float: dir::FloatType) -> bool {
        float
            .roundtrip_f64(value)
            .is_some_and(|rounded| rounded == value)
    }

    /// Decide whether all source union elements assign to a target.
    fn decide_all_union_sources_assignable(
        &mut self,
        sources: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let len = self.union_term_len(sources);

        // require every source variant to be assignable
        for index in 0..len {
            let Some(source) = self.union_term_element(sources, index) else {
                return Ok(Answer::Ready(false));
            };
            let decision = self.decide_type_relation(TypeRelation::Assignable, source, target)?;
            if decision != Answer::Ready(true) {
                return Ok(decision);
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Decide whether one source assigns to any target union element.
    fn decide_any_union_target_assignable(
        &mut self,
        source: TermId<TypeTerm>,
        targets: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let len = self.union_term_len(targets);
        let mut decision = Answer::Ready(false);

        // accept the first matching target
        for index in 0..len {
            let Some(target) = self.union_term_element(targets, index) else {
                return Ok(Answer::Ready(false));
            };

            match self.decide_type_relation(TypeRelation::Assignable, source, target)? {
                Answer::Ready(true) => return Ok(Answer::Ready(true)),
                pending @ Answer::Pending(_) => decision = decision.or(pending),
                Answer::Ready(false) => {}
            }
        }

        Ok(decision)
    }

    /// Return the only union target branch not rejected by assignability.
    fn single_viable_union_target(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let len = self.union_term_len(target);
        let mut selected = None;

        // keep the only branch that may accept the source
        for index in 0..len {
            let Some(element) = self.union_term_element(target, index) else {
                return Ok(None);
            };
            let decision = self.decide_type_relation(TypeRelation::Assignable, source, element)?;
            if decision == Answer::Ready(false) {
                continue;
            }
            if selected.is_some() {
                return Ok(None);
            }

            selected = Some(element);
        }

        self.record_event(CheckEvent::RelationReduce {
            origin,
            relation: TypeRelation::Assignable,
            left: source,
            right: TypeOperand::Term(target),
            reduced_left: None,
            reduced_right: selected,
        });

        Ok(selected)
    }

    /// Return one union term element count.
    fn union_term_len(&self, term: TermId<TypeTerm>) -> usize {
        match self.inference.term(term) {
            TypeTerm::Union { elements } => elements.len(),
            _ => 0,
        }
    }

    /// Return one copied union term element.
    fn union_term_element(&self, term: TermId<TypeTerm>, index: usize) -> Option<TypeOperand> {
        match self.inference.term(term) {
            TypeTerm::Union { elements } => elements.get(index).copied(),
            _ => None,
        }
    }
}
