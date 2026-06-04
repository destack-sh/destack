use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    AwaitTerm, CallCallee, CallTerm, CheckState, ConstructTerm, Decision, FormTerm, FunctionTerm,
    GenericArgument, GenericParameterId, GenericSubstitution, IdentityTerm, ImportMetaTerm,
    IndexSetTerm, IndexTerm, InstanceCheckTerm, KeyMembershipTerm, MemberCallTerm, MemberLookup,
    MemberProjectionOrigin, MemberProtocol, MemberTerm, OperatorTerm, Origin, Progress,
    RangeValueTerm, ReceiverTerm, Reduction, ShapeMember, ShapeTerm, StaticOperand, StaticRelation,
    StaticTerm, Substitution, SuperTerm, TaggedTemplateTerm, TemplateTerm, TermId, TreeTerm,
    TryFailureTerm, TryTerm, TupleElement, TypeOperand, TypeOperationTerm, TypeRelation,
    TypeValueTerm, VariableId, YieldTerm,
};

/// Literal type value with no nested table references.
///
/// Examples:
/// ```ds
/// int32
/// "ready"
/// null
/// ```
#[derive(Debug, Clone, PartialEq)]
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
    /// Return the builtin boolean type.
    pub(in crate::check) fn boolean() -> Self {
        Self::Primitive(dir::PrimitiveType::Boolean)
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
            dir::Type::Literal(literal) => Self::Scalar(literal.clone()),
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
            Self::Scalar(literal) => dir::Type::Literal(literal.clone()),
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
#[derive(Debug, Clone, PartialEq)]
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
        arguments: Vec<GenericArgument>,
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
    RangeValue(TermId<RangeValueTerm>),
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

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();

        match self {
            Self::StaticValue { value } => variables.extend(value.referenced_variables(state)),
            Self::Form { form, payload } => {
                variables.extend(payload.referenced_variables(state));
                variables.extend(state.inference.term(*form).referenced_variables(state));
            }
            Self::Reference {
                origin: _,
                symbol: _,
                arguments,
            } => {
                variables.extend(
                    arguments
                        .iter()
                        .flat_map(|argument| argument.referenced_variables(state)),
                );
            }
            Self::Array { element } => variables.extend(element.referenced_variables(state)),
            Self::Member(member) => {
                variables.extend(state.inference.term(*member).referenced_variables(state));
            }
            Self::FixedArray { element, length } => {
                variables.extend(element.referenced_variables(state));
                variables.extend(length.referenced_variables(state));
            }
            Self::Slice { element } => variables.extend(element.referenced_variables(state)),
            Self::Tuple { form: _, elements } => {
                for element in elements {
                    variables.extend(element.ty.referenced_variables(state));
                }
            }
            Self::Shape(shape) => {
                let members = &state.inference.term(*shape).members;

                for member in members {
                    variables.extend(member.referenced_variables(state));
                }
            }
            Self::Function(function) => {
                variables.extend(state.inference.term(*function).referenced_variables(state));
            }
            Self::Range {
                start: _,
                end: _,
                is_inclusive: _,
            } => {}
            Self::Union { elements } | Self::Intersection { elements } => {
                variables.extend(
                    elements
                        .iter()
                        .flat_map(|element| element.referenced_variables(state)),
                );
            }
            Self::Operation(operation) => {
                variables.extend(state.inference.term(*operation).referenced_variables(state));
            }
            Self::Call(call) => {
                variables.extend(state.inference.term(*call).referenced_variables(state))
            }
            Self::Construct(construct) => {
                variables.extend(state.inference.term(*construct).referenced_variables(state));
            }
            Self::RangeValue(range) => {
                variables.extend(state.inference.term(*range).referenced_variables(state));
            }
            Self::Tree(tree) => {
                variables.extend(state.inference.term(*tree).referenced_variables(state))
            }
            Self::TypeValue(value) => {
                variables.extend(state.inference.term(*value).referenced_variables(state));
            }
            Self::ImportMeta(_) => {}
            Self::Receiver(receiver) => {
                variables.extend(state.inference.term(*receiver).referenced_variables(state));
            }
            Self::Super(term) => {
                variables.extend(state.inference.term(*term).referenced_variables(state));
            }
            Self::Operator(operator) => {
                variables.extend(state.inference.term(*operator).referenced_variables(state));
            }
            Self::Index(index) => {
                variables.extend(state.inference.term(*index).referenced_variables(state));
            }
            Self::IndexSet(set) => {
                variables.extend(state.inference.term(*set).referenced_variables(state));
            }
            Self::KeyMembership(membership) => {
                variables.extend(
                    state
                        .inference
                        .term(*membership)
                        .referenced_variables(state),
                );
            }
            Self::InstanceCheck(instance) => {
                variables.extend(state.inference.term(*instance).referenced_variables(state));
            }
            Self::Identity(identity) => {
                variables.extend(state.inference.term(*identity).referenced_variables(state));
            }
            Self::Await(awaited) => {
                variables.extend(
                    state
                        .inference
                        .term(*awaited)
                        .value
                        .referenced_variables(state),
                );
            }
            Self::Try(tried) => {
                variables.extend(state.inference.term(*tried).referenced_variables(state));
            }
            Self::Yield(yielded) => {
                variables.extend(state.inference.term(*yielded).referenced_variables(state))
            }
            Self::TryFailure(tried) => {
                variables.extend(state.inference.term(*tried).referenced_variables(state));
            }
            Self::Template(template) => {
                variables.extend(state.inference.term(*template).referenced_variables(state));
            }
            Self::TaggedTemplate(template) => {
                variables.extend(state.inference.term(*template).referenced_variables(state));
            }
            Self::Dynamic { constraint } => {
                variables.extend(constraint.referenced_variables(state))
            }
            Self::Closure {
                function,
                environment,
            } => {
                variables.extend(function.referenced_variables(state));
                variables.extend(environment.referenced_variables(state));
            }
            Self::Literal(_)
            | Self::Intrinsic
            | Self::Type(_)
            | Self::Parameter(_)
            | Self::This => {}
        }

        variables
    }

    /// Return whether this term is stable semantic output.
    pub(in crate::check) fn is_stable(&self, state: &CheckState<'_>) -> bool {
        match self {
            Self::Literal(_)
            | Self::Intrinsic
            | Self::Type(_)
            | Self::Parameter(_)
            | Self::Reference { .. }
            | Self::This
            | Self::Member(_)
            | Self::Form { .. }
            | Self::Dynamic { .. }
            | Self::Array { .. }
            | Self::FixedArray { .. }
            | Self::Range { .. }
            | Self::Slice { .. }
            | Self::Tuple { .. }
            | Self::Shape(_)
            | Self::Function(_)
            | Self::Closure { .. }
            | Self::Union { .. }
            | Self::Intersection { .. } => true,
            Self::Operation(operation) => state.inference.term(*operation).is_stable(),
            Self::StaticValue { .. }
            | Self::Call(_)
            | Self::Construct(_)
            | Self::RangeValue(_)
            | Self::Tree(_)
            | Self::TypeValue(_)
            | Self::ImportMeta(_)
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
            | Self::TaggedTemplate(_) => false,
        }
    }
}

impl CheckState<'_> {
    /// Return the element count when one variable is backed by an array literal.
    fn variable_array_literal_length(&self, variable: VariableId) -> Option<usize> {
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

    /// Expect one array literal length to fit a contextual static length.
    fn expect_array_literal_length(
        &mut self,
        variable: VariableId,
        target: StaticOperand,
    ) -> CompilerResult<Option<Progress>> {
        let Some(length) = self.variable_array_literal_length(variable) else {
            return Ok(None);
        };
        let length = StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(length as i64),
        });

        // relate length holes through ordinary static bounds
        let progress = match target {
            StaticOperand::Variable(target) => match self.static_solution(target)? {
                Some(target) => {
                    let decision =
                        self.decide_static_term_relation(StaticRelation::Equal, &length, &target)?;
                    if decision == Decision::Yes {
                        Progress::Unchanged
                    } else {
                        return Ok(None);
                    }
                }
                None => {
                    let length = self.inference.push_term(length.clone());

                    self.relate_static_equality(length, StaticOperand::Variable(target))?
                }
            },
            StaticOperand::Term(target) => {
                let target = self.inference.term(target);
                let decision =
                    self.decide_static_term_relation(StaticRelation::Equal, &length, target)?;
                if decision == Decision::Yes {
                    Progress::Unchanged
                } else {
                    return Ok(None);
                }
            }
            StaticOperand::Static(_) => return Ok(None),
        };

        Ok(Some(progress))
    }

    /// Expect one source operand to use one contextual target type.
    pub(in crate::check) fn expect_type_operand_assignability(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        expected: TypeOperand,
        expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let Some(term) = self.type_operand_term(source)? else {
            return Ok(Progress::Unchanged);
        };
        if let Some(variable) = source.variable() {
            return self.expect_type_definition(
                origin,
                Some(variable),
                expected,
                expected_term,
                &term,
            );
        }

        self.expect_type_definition(origin, None, expected, expected_term, &term)
    }

    /// Decide one type term relation.
    pub(in crate::check) fn decide_type_term_relation(
        &mut self,
        relation: TypeRelation,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let decision = match relation {
            TypeRelation::Equal => self.decide_type_equal(left, right)?,
            TypeRelation::Assignable => self.decide_type_assignable(left, right)?,
            TypeRelation::Castable => self.decide_type_castable(left, right)?,
            TypeRelation::Satisfies => self.decide_type_satisfies(left, right)?,
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
    ) -> CompilerResult<Decision> {
        let left = left.into();
        let right = right.into();
        if matches!(relation, TypeRelation::Assignable | TypeRelation::Castable)
            && let Some(decision) = self.decide_array_literal_fixed_array_relation(left, right)?
        {
            return Ok(decision);
        }

        let left = match left {
            TypeOperand::Variable(variable) => {
                let Some(term) = self.type_solution(variable)? else {
                    return Ok(Decision::Undecidable);
                };

                term
            }
            TypeOperand::Term(term) => self.inference.term(term).clone(),
            TypeOperand::Type(ty) => TypeTerm::Type(ty),
        };
        let right = match right {
            TypeOperand::Variable(variable) => {
                let Some(term) = self.type_solution(variable)? else {
                    return Ok(Decision::Undecidable);
                };

                term
            }
            TypeOperand::Term(term) => self.inference.term(term).clone(),
            TypeOperand::Type(ty) => TypeTerm::Type(ty),
        };

        self.decide_type_term_relation(relation, &left, &right)
    }

    /// Decide array literal assignability into a fixed array target.
    fn decide_array_literal_fixed_array_relation(
        &mut self,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Option<Decision>> {
        let Some(source_variable) = source.variable() else {
            return Ok(None);
        };
        if self
            .variable_array_literal_length(source_variable)
            .is_none()
        {
            return Ok(None);
        }

        let Some(TypeTerm::Array {
            element: source_element,
        }) = self.type_operand_term(source)?
        else {
            return Ok(None);
        };
        let Some(TypeTerm::FixedArray {
            element: target_element,
            length: target_length,
        }) = self.type_operand_term(target)?
        else {
            return Ok(None);
        };

        let Some(length) = self.decide_array_literal_length(source_variable, target_length)? else {
            return Ok(Some(Decision::Undecidable));
        };
        if length != Decision::Yes {
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
    ) -> CompilerResult<Option<Decision>> {
        let Some(length) = self.variable_array_literal_length(variable) else {
            return Ok(None);
        };
        let Some(target) = self.static_operand_term(target)? else {
            return Ok(Some(Decision::Undecidable));
        };
        let length = StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(length as i64),
        });
        let decision = self.decide_static_term_relation(StaticRelation::Equal, &length, &target)?;

        Ok(Some(decision))
    }

    /// Expect the defining term to satisfy the result type.
    pub(in crate::check) fn expect_type_term(
        &mut self,
        origin: Origin,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push contextual upper bounds into the defining term
        let upper_bounds = self.inference.upper_type_bounds(result);
        for expected in upper_bounds {
            let Some(expected_term) = self.reduce_type_operand(origin, expected)? else {
                continue;
            };

            progress = progress.merge(self.expect_type_definition(
                origin,
                Some(result),
                expected,
                &expected_term,
                term,
            )?);
        }

        Ok(progress)
    }

    /// Reduce one type term when the solver has enough input.
    pub(in crate::check) fn reduce_type_term(
        &mut self,
        origin: Origin,
        term: &TypeTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let module = origin.module();
        let reduction = match term {
            TypeTerm::Type(_) => Reduction::value(term.clone()),
            TypeTerm::Member(member) => {
                let member = self.inference.term(*member).clone();

                match self.reduce_member_term(
                    origin,
                    module,
                    member.origin,
                    member.owner,
                    member.key,
                    &member.arguments,
                )? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Operation(operation) => match self.inference.term(*operation).clone() {
                TypeOperationTerm::Exclude { source, target } => {
                    match self.reduce_exclude_term(module, source, target)? {
                        Some(term) => Reduction::value(term),
                        None => Reduction::pending(),
                    }
                }
                TypeOperationTerm::BestCommon { elements } => {
                    match self.reduce_best_common_term(origin, module, &elements)? {
                        Some(term) => Reduction::value(term),
                        None => Reduction::pending(),
                    }
                }
                TypeOperationTerm::Widen { source } => self.reduce_widen_term(origin, source)?,
                TypeOperationTerm::Intrinsic { item, arguments } => {
                    match self.reduce_memory_term(module, item, &arguments)? {
                        Some(term) => Reduction::value(term),
                        None => Reduction::pending(),
                    }
                }
                TypeOperationTerm::StringMapping { mapping, argument } => {
                    match self.reduce_string_mapping_term(origin, module, mapping, argument)? {
                        Some(term) => Reduction::value(term),
                        None => Reduction::pending(),
                    }
                }
                TypeOperationTerm::Conditional {
                    left,
                    right,
                    then_type,
                    else_type,
                } => match self.reduce_conditional_term(left, right, then_type, else_type)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                },
                TypeOperationTerm::Index { left, index } => {
                    match self.reduce_type_index_term(origin, module, left, index)? {
                        Some(term) => Reduction::value(term),
                        None => Reduction::pending(),
                    }
                }
                TypeOperationTerm::TemplateLiteral { .. }
                | TypeOperationTerm::Infer { .. }
                | TypeOperationTerm::KeyOf { .. }
                | TypeOperationTerm::Mapped { .. } => Reduction::value(term.clone()),
            },
            TypeTerm::Reference {
                origin: reference_origin,
                symbol,
                arguments,
            } => {
                self.reduce_reference_term(origin, module, *reference_origin, *symbol, arguments)?
            }
            TypeTerm::StaticValue { value } => {
                match self.reduce_static_value_term(module, *value)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Call(call) => {
                let call = self.inference.term(*call).clone();

                self.reduce_call_term(origin, module, &call)?
            }
            TypeTerm::Construct(construct) => {
                let construct = self.inference.term(*construct).clone();

                self.reduce_construct_term(origin, module, &construct)?
            }
            TypeTerm::RangeValue(range) => {
                let range = self.inference.term(*range).clone();

                self.reduce_range_value_term(module, &range)?
            }
            TypeTerm::Tree(tree) => {
                let tree = self.inference.term(*tree).clone();

                self.reduce_tree_term(module, &tree)?
            }
            TypeTerm::TypeValue(value) => {
                let value = self.inference.term(*value).clone();

                self.reduce_type_value_term(module, &value)?
            }
            TypeTerm::ImportMeta(meta) => {
                let meta = self.inference.term(*meta).clone();

                self.reduce_import_meta_term(module, &meta)?
            }
            TypeTerm::Receiver(receiver) => {
                let receiver = self.inference.term(*receiver).clone();

                self.reduce_receiver_term(&receiver)?
            }
            TypeTerm::Super(term) => {
                let term = self.inference.term(*term).clone();

                self.reduce_super_term(module, &term)?
            }
            TypeTerm::Operator(operator) => {
                let operator = self.inference.term(*operator).clone();

                match self.reduce_operator_term(origin, &operator)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Index(index) => {
                let index = self.inference.term(*index).clone();

                self.reduce_index_term(origin, module, &index)?
            }
            TypeTerm::IndexSet(set) => {
                let set = self.inference.term(*set).clone();

                self.reduce_index_set_term(origin, module, &set)?
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = self.inference.term(*membership).clone();

                match self.reduce_key_membership_term(&membership)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = self.inference.term(*instance).clone();

                match self.reduce_instance_check_term(&instance)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Identity(identity) => {
                let identity = self.inference.term(*identity).clone();

                match self.reduce_identity_term(&identity)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Template(template) => {
                let template = self.inference.term(*template).clone();

                match self.reduce_template_term(&template)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::TaggedTemplate(template) => {
                let template = self.inference.term(*template).clone();

                self.reduce_tagged_template_term(origin, &template)?
            }
            TypeTerm::Await(awaited) => {
                let awaited = self.inference.term(*awaited).clone();

                match self.reduce_await_term(&awaited)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Try(tried) => {
                let tried = self.inference.term(*tried).clone();

                match self.reduce_try_term(origin, module, &tried)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.inference.term(*yielded).clone();

                match self.reduce_yield_term(&yielded)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::TryFailure(tried) => {
                let tried = self.inference.term(*tried).clone();

                match self.reduce_try_failure_term(origin, module, &tried)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Form { form, payload } => {
                let Some(payload) = self.reduce_type_operand_to_operand(origin, *payload)? else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::Form {
                    form: *form,
                    payload,
                })
            }
            TypeTerm::Array { element } => {
                let Some(element) = self.reduce_type_operand_to_operand(origin, *element)? else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::Array { element })
            }
            TypeTerm::FixedArray { element, length } => {
                let Some(element) = self.reduce_type_operand_to_operand(origin, *element)? else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::FixedArray {
                    element,
                    length: *length,
                })
            }
            TypeTerm::Slice { element } => {
                let Some(element) = self.reduce_type_operand_to_operand(origin, *element)? else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::Slice { element })
            }
            TypeTerm::Tuple { form, elements } => {
                let Some(elements) = self.reduce_tuple_term(origin, elements)? else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::Tuple {
                    form: *form,
                    elements,
                })
            }
            TypeTerm::Union { elements } => {
                let Some(elements) = self.reduce_type_operands_to_operands(origin, elements)?
                else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::Union { elements })
            }
            TypeTerm::Intersection { elements } => {
                let Some(elements) = self.reduce_type_operands_to_operands(origin, elements)?
                else {
                    return Ok(Reduction::pending());
                };

                Reduction::value(TypeTerm::Intersection { elements })
            }
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::Parameter(_)
            | TypeTerm::This
            | TypeTerm::Function(_)
            | TypeTerm::Range { .. }
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Closure { .. } => Reduction::value(term.clone()),
            TypeTerm::Shape(shape) => {
                let members = self.inference.term(*shape).members.clone();
                let Some(members) = self.reduce_shape_term(origin, &members)? else {
                    return Ok(Reduction::pending());
                };
                if members.as_slice() == self.inference.term(*shape).members.as_slice() {
                    return Ok(Reduction::value(TypeTerm::Shape(*shape)));
                }

                Reduction::value(self.push_shape_type(members))
            }
        };

        Ok(reduction)
    }

    /// Reduce nested type operands.
    fn reduce_type_operands_to_operands(
        &mut self,
        origin: Origin,
        operands: &[TypeOperand],
    ) -> CompilerResult<Option<Vec<TypeOperand>>> {
        let mut reduced = Vec::with_capacity(operands.len());

        // reduce operands in source order
        for operand in operands {
            let Some(operand) = self.reduce_type_operand_to_operand(origin, *operand)? else {
                return Ok(None);
            };

            reduced.push(operand);
        }

        Ok(Some(reduced))
    }

    /// Reduce nested operands inside one tuple.
    fn reduce_tuple_term(
        &mut self,
        origin: Origin,
        elements: &[TupleElement],
    ) -> CompilerResult<Option<Vec<TupleElement>>> {
        let mut reduced = Vec::with_capacity(elements.len());

        // reduce elements in source order
        for element in elements {
            let Some(ty) = self.reduce_type_operand_to_operand(origin, element.ty)? else {
                return Ok(None);
            };

            reduced.push(TupleElement { ty, ..*element });
        }

        Ok(Some(reduced))
    }

    /// Reduce nested operands inside one structural shape.
    fn reduce_shape_term(
        &mut self,
        origin: Origin,
        members: &[ShapeMember],
    ) -> CompilerResult<Option<SmallVec<[ShapeMember; 2]>>> {
        let mut reduced = SmallVec::with_capacity(members.len());

        // reduce members in source order
        for member in members {
            let Some(member) = self.reduce_shape_member(origin, member)? else {
                return Ok(None);
            };

            reduced.push(member);
        }

        Ok(Some(reduced))
    }

    /// Reduce nested operands inside one shape member.
    fn reduce_shape_member(
        &mut self,
        origin: Origin,
        member: &ShapeMember,
    ) -> CompilerResult<Option<ShapeMember>> {
        let member = match member {
            ShapeMember::Field {
                key,
                ty,
                is_optional,
                is_readonly,
            } => ShapeMember::Field {
                key: *key,
                ty: match self.reduce_type_operand_to_operand(origin, *ty)? {
                    Some(ty) => ty,
                    None => return Ok(None),
                },
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
            ShapeMember::Spread { .. } => return Ok(None),
            ShapeMember::CallSignature { ty } => ShapeMember::CallSignature {
                ty: match self.reduce_type_operand_to_operand(origin, *ty)? {
                    Some(ty) => ty,
                    None => return Ok(None),
                },
            },
            ShapeMember::ConstructSignature { ty } => ShapeMember::ConstructSignature {
                ty: match self.reduce_type_operand_to_operand(origin, *ty)? {
                    Some(ty) => ty,
                    None => return Ok(None),
                },
            },
            ShapeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => ShapeMember::IndexSignature {
                name: *name,
                key_type: match self.reduce_type_operand_to_operand(origin, *key_type)? {
                    Some(key_type) => key_type,
                    None => return Ok(None),
                },
                value_type: match self.reduce_type_operand_to_operand(origin, *value_type)? {
                    Some(value_type) => value_type,
                    None => return Ok(None),
                },
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
        };

        Ok(Some(member))
    }

    /// Reduce one term operand while preserving variable operands.
    fn reduce_type_operand_to_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let TypeOperand::Term(term) = operand else {
            return Ok(Some(operand));
        };
        let original = self.inference.term(term).clone();
        let Some(reduced) = self.reduce_type_operand_term(origin, original.clone())? else {
            return Ok(None);
        };
        if reduced == original {
            return Ok(Some(operand));
        }
        let reduced = self.inference.push_term(reduced);

        Ok(Some(reduced.into()))
    }

    /// Decompose equality between solved type terms into smaller relations.
    pub(in crate::check) fn constrain_solved_type_equal(
        &mut self,
        origin: Origin,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (left, right) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), right) if right.is_unit() => {
                Progress::Unchanged
            }
            (left, TypeTerm::Literal(TypeLiteralTerm::Void)) if left.is_unit() => {
                Progress::Unchanged
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
                let left_form = self.inference.term(*left_form).clone();
                let right_form = self.inference.term(*right_form).clone();
                let form = self.constrain_form_equal(&left_form, &right_form)?;
                let value = self.relate_type_equality(origin, *left_value, *right_value)?;

                form.merge(value)
            }
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
            ) => self.relate_type_equality(origin, *left_element, *right_element)?,
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
                let element = self.relate_type_equality(origin, *left_element, *right_element)?;
                let length = self.relate_static_equality(*left_length, *right_length)?;

                element.merge(length)
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right,
                },
            ) => {
                if left_form == right_form {
                    self.constrain_tuple_elements_equal(origin, left, right)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape(left), TypeTerm::Shape(right)) => {
                let left = self.inference.term(*left).members.clone();
                let right = self.inference.term(*right).members.clone();

                self.constrain_shape_members_equal(origin, &left, &right)?
            }
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
            ) if left_symbol == right_symbol => {
                self.constrain_argument_list_equal(origin, left_arguments, right_arguments)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decompose assignability between solved type terms into smaller relations.
    pub(in crate::check) fn constrain_solved_type_assignable(
        &mut self,
        origin: Origin,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (source, target) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), target) if target.is_unit() => {
                Progress::Unchanged
            }
            (source, TypeTerm::Literal(TypeLiteralTerm::Void)) if source.is_unit() => {
                Progress::Unchanged
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
                let source_form = self.inference.term(*source_form).clone();
                let target_form = self.inference.term(*target_form).clone();
                let form = self.constrain_form_assignable(&source_form, &target_form)?;
                let value = self.relate_type_assignability(origin, *source_value, *target_value)?;

                form.merge(value)
            }
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
            ) => self.relate_type_assignability(origin, *source_element, *target_element)?,
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: _,
                },
                TypeTerm::Slice {
                    element: target_element,
                },
            ) => self.relate_type_assignability(origin, *source_element, *target_element)?,
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
                let element =
                    self.relate_type_assignability(origin, *source_element, *target_element)?;
                let length = self.relate_static_equality(*source_length, *target_length)?;

                element.merge(length)
            }
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: source,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: target,
                },
            ) => {
                if source_form == target_form {
                    self.constrain_tuple_elements_assignable(origin, source, target)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape(source), TypeTerm::Shape(target)) => {
                let source = self.inference.term(*source).members.clone();
                let target = self.inference.term(*target).members.clone();

                self.constrain_shape_members_assignable(origin, &source, &target)?
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
                self.constrain_argument_list_equal(origin, source_arguments, target_arguments)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }
}

impl TypeTerm {
    /// Substitute generic arguments through this type term.
    pub(in crate::check) fn substitute<'a>(
        &self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let substitution = substitution.into();
        let term = match self {
            TypeTerm::Parameter(parameter_id) => {
                if let Some(argument) = state.substitution_type_operand(substitution, *parameter_id)
                {
                    let Some(term) = state.type_operand_term(argument)? else {
                        return Ok(None);
                    };

                    term
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
                        state.substitution_type_symbol_operand(substitution, *symbol)
                {
                    let Some(term) = state.type_operand_term(argument)? else {
                        return Ok(None);
                    };

                    term
                } else {
                    TypeTerm::Reference {
                        origin: *reference_origin,
                        symbol: *symbol,
                        arguments: state
                            .substitute_arguments(module, substitution, arguments)?
                            .into_vec(),
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
                let members = state.inference.term(*shape).members.clone();
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
                            receiver: state.substitute_type_operand(
                                module,
                                substitution,
                                member.receiver,
                            )?,
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
                    arguments: state
                        .substitute_type_operands(module, substitution, &call.arguments)?
                        .into(),
                    argument_values: call.argument_values.clone(),
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
                    arguments: state
                        .substitute_type_operands(module, substitution, &construct.arguments)?
                        .into(),
                };
                let construct = state.inference.push_term(construct);

                TypeTerm::Construct(construct)
            }
            TypeTerm::RangeValue(range) => {
                let range = state.inference.term(*range).clone();
                let range = RangeValueTerm {
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
                let value = state.inference.term(*value).clone();
                let value = TypeValueTerm {
                    source: value.source,
                    ty: state.substitute_type_operand(module, substitution, value.ty)?,
                };
                let value = state.inference.push_term(value);

                TypeTerm::TypeValue(value)
            }
            TypeTerm::ImportMeta(meta) => TypeTerm::ImportMeta(*meta),
            TypeTerm::Receiver(receiver) => {
                let receiver = state.inference.term(*receiver).clone();
                let receiver = ReceiverTerm {
                    source: receiver.source,
                    kind: receiver.kind,
                    ty: state.substitute_type_operand(module, substitution, receiver.ty)?,
                };
                let receiver = state.inference.push_term(receiver);

                TypeTerm::Receiver(receiver)
            }
            TypeTerm::Super(term) => {
                let term = state.inference.term(*term).clone();
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
                let operator = state.inference.term(*operator).clone();
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
                let index = state.inference.term(*index).clone();
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
                let set = state.inference.term(*set).clone();
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
                let membership = state.inference.term(*membership).clone();
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
                let instance = state.inference.term(*instance).clone();
                let instance = InstanceCheckTerm {
                    source: instance.source,
                    value: state.substitute_type_operand(module, substitution, instance.value)?,
                    target: state.substitute_type_operand(module, substitution, instance.target)?,
                };
                let instance = state.inference.push_term(instance);

                TypeTerm::InstanceCheck(instance)
            }
            TypeTerm::Identity(identity) => {
                let identity = state.inference.term(*identity).clone();
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
                let awaited = state.inference.term(*awaited).clone();
                let awaited = AwaitTerm {
                    source: awaited.source,
                    value: state.substitute_type_operand(module, substitution, awaited.value)?,
                };
                let awaited = state.inference.push_term(awaited);

                TypeTerm::Await(awaited)
            }
            TypeTerm::Try(tried) => {
                let tried = state.inference.term(*tried).clone();
                let tried = TryTerm {
                    source: tried.source,
                    value: state.substitute_type_operand(module, substitution, tried.value)?,
                    kind: tried.kind,
                };
                let tried = state.inference.push_term(tried);

                TypeTerm::Try(tried)
            }
            TypeTerm::Yield(yielded) => {
                let yielded = state.inference.term(*yielded).clone();
                let yielded = YieldTerm {
                    source: yielded.source,
                    value: yielded
                        .value
                        .map(|value| state.substitute_type_operand(module, substitution, value))
                        .transpose()?,
                    yield_target: yielded
                        .yield_target
                        .map(|ty| state.substitute_type_variable(module, substitution, ty))
                        .transpose()?,
                    resume_target: yielded
                        .resume_target
                        .map(|ty| state.substitute_type_variable(module, substitution, ty))
                        .transpose()?,
                    delegate_return_target: yielded
                        .delegate_return_target
                        .map(|ty| state.substitute_type_variable(module, substitution, ty))
                        .transpose()?,
                    cardinality: yielded.cardinality,
                };
                let yielded = state.inference.push_term(yielded);

                TypeTerm::Yield(yielded)
            }
            TypeTerm::TryFailure(tried) => {
                let tried = state.inference.term(*tried).clone();
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
            TypeTerm::This => {
                let Some(receiver) = substitution.receiver else {
                    return Ok(Some(self.clone()));
                };

                let Some(term) = state.type_operand_term(receiver.receiver)? else {
                    return Ok(None);
                };

                term
            }
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::Type(_)
            | TypeTerm::Range { .. } => self.clone(),
        };

        Ok(Some(term))
    }
}

impl CheckState<'_> {
    /// Decide exact equality for type variable lists.
    pub(in crate::check) fn decide_type_variable_list_equal<L, R>(
        &mut self,
        left: &[L],
        right: &[R],
    ) -> CompilerResult<Decision>
    where
        L: Copy + Into<TypeOperand>,
        R: Copy + Into<TypeOperand>,
    {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare matching variables in declaration order
        for (left, right) in left.iter().zip(right) {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Equal,
                (*left).into(),
                (*right).into(),
            )?);
            if decision == Decision::No {
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
    ) -> CompilerResult<Decision> {
        let decision = match (left, right) {
            (Some(left), Some(right)) => {
                self.decide_type_relation(TypeRelation::Equal, left, right)?
            }
            (None, None) => Decision::Yes,
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Expect one defining term to satisfy one result type.
    fn expect_type_definition(
        &mut self,
        origin: Origin,
        result: Option<VariableId>,
        expected: TypeOperand,
        expected_term: &TypeTerm,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match term {
            TypeTerm::Literal(_) | TypeTerm::Intrinsic | TypeTerm::Type(_) => Progress::Unchanged,
            TypeTerm::Form { form, payload } => {
                let TypeTerm::Form {
                    form: result_form,
                    payload: result_payload,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let form_term = self.inference.term(*form).clone();
                let result_form = self.inference.term(*result_form).clone();
                let form = self.expect_form_term(&form_term, &result_form)?;
                let payload =
                    self.relate_contextual_type_assignability(origin, *payload, *result_payload)?;

                form.merge(payload)
            }
            TypeTerm::Array { element } => match expected_term {
                TypeTerm::Array {
                    element: result_element,
                }
                | TypeTerm::Slice {
                    element: result_element,
                } => {
                    self.relate_contextual_type_assignability(origin, *element, *result_element)?
                }
                TypeTerm::FixedArray {
                    element: result_element,
                    length: result_length,
                } => {
                    let Some(result) = result else {
                        return Ok(Progress::Unchanged);
                    };
                    let Some(length) =
                        self.expect_array_literal_length(result, (*result_length).into())?
                    else {
                        return Ok(Progress::Unchanged);
                    };
                    let element = self.relate_contextual_type_assignability(
                        origin,
                        *element,
                        *result_element,
                    )?;

                    length.merge(element)
                }
                _ => Progress::Unchanged,
            },
            TypeTerm::Slice { element } => {
                let TypeTerm::Slice {
                    element: result_element,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.relate_contextual_type_assignability(origin, *element, *result_element)?
            }
            TypeTerm::FixedArray { element, length } => {
                let TypeTerm::FixedArray {
                    element: result_element,
                    length: result_length,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let element =
                    self.relate_contextual_type_assignability(origin, *element, *result_element)?;
                let length = self.relate_static_equality(*length, *result_length)?;

                element.merge(length)
            }
            TypeTerm::Tuple { form: _, elements } => {
                let TypeTerm::Tuple {
                    form: _,
                    elements: result_elements,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_tuple_element_terms(origin, elements, &result_elements)?
            }
            TypeTerm::Shape(shape) => {
                let TypeTerm::Shape(result) = expected_term else {
                    return Ok(Progress::Unchanged);
                };
                let members = self.inference.term(*shape).members.clone();
                let result_members = self.inference.term(*result).members.clone();

                self.expect_shape_member_terms(origin, &members, &result_members)?
            }
            TypeTerm::Call(call) => {
                let call = self.inference.term(*call).clone();
                let Some(expected) = expected.variable().or(result) else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_call_term(origin, &call, expected)?
            }
            TypeTerm::Construct(construct) => {
                let construct = self.inference.term(*construct).clone();
                let Some(expected) = expected.variable().or(result) else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_construct_term(origin, &construct, expected)?
            }
            TypeTerm::Operator(operator) => {
                let operator = self.inference.term(*operator).clone();
                let Some(expected) = expected.variable().or(result) else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_operator_term(origin, &operator, expected, expected_term)?
            }
            TypeTerm::Index(index) => {
                let index = self.inference.term(*index).clone();
                let Some(expected) = expected.variable().or(result) else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_index_term(origin, &index, expected)?
            }
            TypeTerm::IndexSet(set) => {
                let set = self.inference.term(*set).clone();
                let Some(expected) = expected.variable().or(result) else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_index_set_term(origin, &set, expected)?
            }
            TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_) => Progress::Unchanged,
            TypeTerm::TaggedTemplate(template) => {
                let template = self.inference.term(*template).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_tagged_template_term(origin, &template, expected)?
            }
            TypeTerm::Await(awaited) => {
                let awaited = self.inference.term(*awaited).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_await_term(origin, &awaited, expected)?
            }
            TypeTerm::Try(tried) => {
                let tried = self.inference.term(*tried).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_try_term(origin, &tried, expected)?
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.inference.term(*yielded).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_yield_term(origin, &yielded, expected)?
            }
            TypeTerm::TryFailure(tried) => {
                let tried = self.inference.term(*tried).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_try_failure_term(origin, &tried, expected)?
            }
            TypeTerm::Operation(operation) => match self.inference.term(*operation).clone() {
                TypeOperationTerm::Exclude { source, target: _ } => {
                    self.relate_contextual_type_assignability(origin, source, expected)?
                }
                TypeOperationTerm::BestCommon { elements } => self.expect_best_common_term(
                    origin,
                    result,
                    &elements,
                    expected,
                    expected_term,
                )?,
                TypeOperationTerm::Conditional {
                    left,
                    right,
                    then_type,
                    else_type,
                } => self
                    .expect_conditional_term(origin, left, right, then_type, else_type, expected)?,
                TypeOperationTerm::Widen { source } => {
                    self.expect_widen_term(origin, source, expected, expected_term)?
                }
                TypeOperationTerm::Index { left, index } => self.expect_type_index_term(
                    origin,
                    result
                        .map(|result| result.module)
                        .unwrap_or_else(|| origin.module()),
                    left,
                    index,
                    expected,
                )?,
                TypeOperationTerm::TemplateLiteral { .. }
                | TypeOperationTerm::Infer { .. }
                | TypeOperationTerm::KeyOf { .. }
                | TypeOperationTerm::Mapped { .. }
                | TypeOperationTerm::StringMapping { .. }
                | TypeOperationTerm::Intrinsic { .. } => Progress::Unchanged,
            },
            TypeTerm::RangeValue(range) => {
                let range = self.inference.term(*range).clone();

                self.expect_range_value_term(origin, &range, expected_term)?
            }
            TypeTerm::Function(function) => {
                let TypeTerm::Function(expected_function) = expected_term else {
                    return Ok(Progress::Unchanged);
                };

                let function = self.inference.term(*function).clone();
                let expected_function = self.inference.term(*expected_function).clone();

                self.expect_function_term(origin, &function, &expected_function)?
            }
            TypeTerm::Member(_)
            | TypeTerm::Reference { .. }
            | TypeTerm::StaticValue { .. }
            | TypeTerm::Range { .. }
            | TypeTerm::Tree(_)
            | TypeTerm::TypeValue(_)
            | TypeTerm::ImportMeta(_)
            | TypeTerm::Receiver(_)
            | TypeTerm::Super(_)
            | TypeTerm::Union { .. }
            | TypeTerm::Intersection { .. }
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Closure { .. }
            | TypeTerm::Parameter(_)
            | TypeTerm::This => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decide whether an explicit cast is valid.
    fn decide_type_castable(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let forward = self.decide_type_assignable(source, target)?;
        if forward == Decision::Yes {
            return Ok(Decision::Yes);
        }
        let backward = self.decide_type_assignable(target, source)?;
        if backward == Decision::Yes {
            return Ok(Decision::Yes);
        }
        if forward == Decision::Undecidable || backward == Decision::Undecidable {
            return Ok(Decision::Undecidable);
        }

        Ok(Decision::No)
    }

    /// Decide exact type equality.
    fn decide_type_equal(&mut self, left: &TypeTerm, right: &TypeTerm) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), right) if right.is_unit() => Decision::Yes,
            (left, TypeTerm::Literal(TypeLiteralTerm::Void)) if left.is_unit() => Decision::Yes,
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
                if form != Decision::Yes {
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
                    arguments: left_arguments,
                },
                TypeTerm::Reference {
                    origin: _,
                    symbol: right_symbol,
                    arguments: right_arguments,
                },
            ) => {
                if left_symbol != right_symbol {
                    Decision::No
                } else {
                    self.decide_argument_list_equal(left_arguments, right_arguments)?
                }
            }
            (TypeTerm::Array { element: left }, TypeTerm::Array { element: right }) => {
                self.decide_type_relation(TypeRelation::Equal, *left, *right)?
            }
            (
                TypeTerm::Slice {
                    element: left_element,
                },
                TypeTerm::Slice {
                    element: right_element,
                },
            ) => self.decide_type_relation(TypeRelation::Equal, *left_element, *right_element)?,
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
                let element =
                    self.decide_type_relation(TypeRelation::Equal, *left_element, *right_element)?;
                let length = self.decide_static_relation(
                    StaticRelation::Equal,
                    *left_length,
                    *right_length,
                )?;

                element.and(length)
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left_elements,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right_elements,
                },
            ) => {
                if left_form != right_form {
                    Decision::No
                } else {
                    self.decide_tuple_elements_equal(left_elements, right_elements)?
                }
            }
            (TypeTerm::Shape(left), TypeTerm::Shape(right)) => {
                let left = self.inference.term(*left).members.clone();
                let right = self.inference.term(*right).members.clone();

                self.decide_shape_members_equal(&left, &right)?
            }
            (TypeTerm::Function(left), TypeTerm::Function(right)) => {
                let left = self.inference.term(*left).clone();
                let right = self.inference.term(*right).clone();

                self.decide_function_equal(&left, &right)?
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
                    Decision::Yes
                } else {
                    Decision::No
                }
            }
            (TypeTerm::Union { elements: left }, TypeTerm::Union { elements: right })
            | (
                TypeTerm::Intersection { elements: left },
                TypeTerm::Intersection { elements: right },
            ) => self.decide_type_variable_list_equal(left, right)?,
            _ => Decision::No,
        };

        Ok(decision)
    }

    /// Decide assignability from source to target.
    fn decide_type_assignable(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_type_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), target) if target.is_unit() => Decision::Yes,
            (source, TypeTerm::Literal(TypeLiteralTerm::Void)) if source.is_unit() => Decision::Yes,
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
                let form = self.decide_form_assignable(
                    self.inference.term(*source_form),
                    self.inference.term(*target_form),
                )?;
                if form != Decision::Yes {
                    return Ok(form);
                }

                self.decide_type_relation(TypeRelation::Assignable, *source_value, *target_value)?
            }
            (TypeTerm::Union { elements }, target) => {
                self.decide_all_sources_assignable(elements, target)?
            }
            (source, TypeTerm::Union { elements }) => {
                self.decide_any_target_assignable(source, elements)?
            }
            (TypeTerm::Literal(source), TypeTerm::Literal(target)) => {
                self.decide_type_atom_assignable(source, target)
            }
            (TypeTerm::Array { element: source }, TypeTerm::Array { element: target }) => {
                self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
            }
            (TypeTerm::Array { element: source }, TypeTerm::Slice { element: target }) => {
                self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
            }
            (TypeTerm::Slice { element: source }, TypeTerm::Slice { element: target }) => {
                self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
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
                let element = self.decide_type_relation(
                    TypeRelation::Assignable,
                    *source_element,
                    *target_element,
                )?;
                let length = self.decide_static_relation(
                    StaticRelation::Equal,
                    *source_length,
                    *target_length,
                )?;

                element.and(length)
            }
            (
                TypeTerm::FixedArray {
                    element: source, ..
                },
                TypeTerm::Slice { element: target },
            ) => self.decide_type_relation(TypeRelation::Assignable, *source, *target)?,
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: source_elements,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: target_elements,
                },
            ) => {
                if source_form != target_form {
                    Decision::No
                } else {
                    self.decide_tuple_elements_assignable(source_elements, target_elements)?
                }
            }
            (TypeTerm::Shape(source), TypeTerm::Shape(target)) => {
                let source = self.inference.term(*source).members.clone();
                let target = self.inference.term(*target).members.clone();

                self.decide_shape_assignable(&source, &target)?
            }
            _ => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide whether source satisfies one target constraint.
    fn decide_type_satisfies(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_type_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (TypeTerm::Shape(source), TypeTerm::Shape(target)) => {
                let source = self.inference.term(*source).members.clone();
                let target = self.inference.term(*target).members.clone();

                self.decide_shape_satisfies(&source, &target)?
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
            ) => self.decide_nominal_satisfies(
                *source_symbol,
                source_arguments,
                *target_symbol,
                target_arguments,
            )?,
            _ => self.decide_type_assignable(source, target)?,
        };

        Ok(decision)
    }

    /// Decide whether one nominal reference satisfies another nominal constraint.
    fn decide_nominal_satisfies(
        &mut self,
        source_symbol: dir::GlobalSymbolId,
        source_arguments: &[GenericArgument],
        target_symbol: dir::GlobalSymbolId,
        target_arguments: &[GenericArgument],
    ) -> CompilerResult<Decision> {
        if !self.symbol_kind(target_symbol).is_interface() {
            return Ok(Decision::Undecidable);
        }
        let target_members = self.named_member_symbols(target_symbol);
        let target_substitution = self.generic_substitution(target_symbol, target_arguments)?;
        let source = TypeTerm::Reference {
            origin: Origin::Symbol(source_symbol),
            symbol: source_symbol,
            arguments: source_arguments.to_vec().into(),
        };
        let mut decision = Decision::Yes;

        // require every interface member from the source nominal type
        for (key, target_member) in target_members {
            let member = self.decide_nominal_member_satisfies(
                &source,
                key,
                target_member,
                &target_substitution,
            )?;

            decision = decision.and(member);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source nominal member satisfies one target member.
    fn decide_nominal_member_satisfies(
        &mut self,
        source: &TypeTerm,
        key: dir::StaticKey,
        target_member: dir::GlobalSymbolId,
        target_substitution: &GenericSubstitution,
    ) -> CompilerResult<Decision> {
        let target_member_symbol = target_member;
        let candidates = self.lookup_type_member(
            Origin::Symbol(target_member),
            target_member.module_id,
            source,
            &key,
        )?;
        let MemberLookup::Found(mut candidates) = candidates else {
            return Ok(Decision::No);
        };
        if candidates.len() != 1 {
            return Ok(Decision::Undecidable);
        }
        let source_member = candidates.remove(0);
        let target_member =
            self.import_symbol_type_operand(target_member.module_id, target_member)?;
        let Some(target_member) = self.type_operand_term(target_member)? else {
            return Ok(Decision::Undecidable);
        };
        let target_member = if target_substitution.is_empty() {
            target_member
        } else {
            let Some(term) = target_member.substitute(
                target_member_symbol.module_id,
                target_substitution,
                self,
            )?
            else {
                return Ok(Decision::Undecidable);
            };

            term
        };

        self.decide_member_type_satisfies(&source_member.ty, &target_member)
    }

    /// Decide whether one member type satisfies another member type.
    fn decide_member_type_satisfies(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        match (source, target) {
            (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                let source = self.inference.term(*source).clone();
                let target = self.inference.term(*target).clone();

                self.decide_member_function_satisfies(&source, &target)
            }
            _ => self.decide_type_satisfies(source, target),
        }
    }

    /// Decide whether one member function satisfies another member function.
    fn decide_member_function_satisfies(
        &mut self,
        source: &FunctionTerm,
        target: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if source.asynchrony != target.asynchrony || source.is_generator != target.is_generator {
            return Ok(Decision::No);
        }

        self.decide_member_function_signature_satisfies(source, target)
    }

    /// Decide whether one member function signature satisfies another.
    fn decide_member_function_signature_satisfies(
        &mut self,
        source: &FunctionTerm,
        target: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if source.parameters.len() != target.parameters.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare runtime parameters contravariantly
        for (source, target) in source.parameters.iter().zip(&target.parameters) {
            if source.is_optional != target.is_optional || source.is_rest != target.is_rest {
                return Ok(Decision::No);
            }

            decision = decision.and(self.decide_type_relation(
                TypeRelation::Satisfies,
                target.ty,
                source.ty,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        // compare return types covariantly
        match (source.return_type, target.return_type) {
            (Some(source), Some(target)) => {
                decision = decision.and(self.decide_type_relation(
                    TypeRelation::Satisfies,
                    source,
                    target,
                )?);
            }
            (None, None) => {}
            _ => return Ok(Decision::No),
        }

        Ok(decision)
    }

    /// Reduce one type declaration reference.
    fn reduce_reference_term(
        &mut self,
        _origin: Origin,
        _module: ModuleId,
        reference_origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Reduction<TypeTerm>> {
        Ok(Reduction::value(TypeTerm::Reference {
            origin: reference_origin,
            symbol,
            arguments: arguments.to_vec().into(),
        }))
    }

    /// Reduce one static value used as a type.
    fn reduce_static_value_term(
        &mut self,
        module: ModuleId,
        value: StaticOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let (value_module, term, origin) = match value {
            StaticOperand::Variable(variable) => {
                let Some(term) = self.static_solution(variable)? else {
                    return Ok(None);
                };

                (variable.module, term, Some(self.variable(variable).source))
            }
            StaticOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                (module, term, None)
            }
            StaticOperand::Static(value) => {
                let term = self.r#static(value).clone();

                (value.module_id, StaticTerm::Literal(term), None)
            }
        };
        let term = if let Some(origin) = origin {
            let Some(term) = self.reduce_static_term(origin, &term)? else {
                return Ok(None);
            };

            term
        } else {
            term
        };
        let Some(term) = self.type_from_static_term(module, value_module, &term)? else {
            return Ok(None);
        };

        Ok(Some(term))
    }

    /// Return the singleton type described by one static value.
    fn type_from_static_term(
        &mut self,
        module: ModuleId,
        value_module: ModuleId,
        term: &StaticTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match term {
            StaticTerm::Static(value) => {
                let term = self.r#static(*value).clone();

                return self.type_from_static_term(
                    module,
                    value.module_id,
                    &StaticTerm::Literal(term),
                );
            }
            StaticTerm::Expression(expression) => {
                let Some(term) = self.build_static_expression_term(expression.clone())? else {
                    return Ok(None);
                };
                let origin = Origin::Node(expression.clone().into_any());
                let Some(term) = self.reduce_static_term(origin, &term)? else {
                    return Ok(None);
                };

                return self.type_from_static_term(module, expression.module_id, &term);
            }
            StaticTerm::Literal(dir::StaticTerm::ScalarLiteral { value }) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            StaticTerm::Literal(dir::StaticTerm::Type { ty }) => {
                let operand = self.import_type_operand(module, *ty)?;
                let Some(term) = self.type_operand_term(operand)? else {
                    return Ok(None);
                };

                term
            }
            StaticTerm::Literal(dir::StaticTerm::TypeLiteral { value }) => {
                let ty = dir::Type::from(value.clone());
                let Some(literal) = TypeLiteralTerm::from_type(&ty) else {
                    return Ok(None);
                };

                TypeTerm::Literal(literal)
            }
            StaticTerm::Parameter(parameter) => TypeTerm::Parameter(*parameter),
            StaticTerm::Literal(dir::StaticTerm::Object { properties }) => {
                let mut members = Vec::with_capacity(properties.len());
                for property in properties {
                    let dir::StaticProperty::Field { key, value } = property else {
                        return Ok(None);
                    };
                    let Some(term) = self.type_from_static_term(
                        module,
                        value_module,
                        &StaticTerm::Literal(value.clone()),
                    )?
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
            StaticTerm::Literal(_) => return Ok(None),
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

    /// Decide exact atom equality.
    fn decide_type_literal_equal(
        &self,
        left: &TypeLiteralTerm,
        right: &TypeLiteralTerm,
    ) -> Decision {
        if left == right {
            Decision::Yes
        } else {
            Decision::No
        }
    }

    /// Decide atom assignability.
    fn decide_type_atom_assignable(
        &self,
        source: &TypeLiteralTerm,
        target: &TypeLiteralTerm,
    ) -> Decision {
        if source == target {
            return Decision::Yes;
        }

        match (source, target) {
            (TypeLiteralTerm::Error, _) | (_, TypeLiteralTerm::Error) => Decision::Yes,
            (TypeLiteralTerm::Any, _) | (_, TypeLiteralTerm::Any) => Decision::Yes,
            (TypeLiteralTerm::Never, _) => Decision::Yes,
            (_, TypeLiteralTerm::Unknown) => Decision::Yes,
            (TypeLiteralTerm::Unknown, _) => Decision::No,
            (TypeLiteralTerm::Scalar(literal), target) => {
                self.decide_literal_assignable(literal, target)
            }
            _ => Decision::No,
        }
    }

    /// Decide literal widening assignability.
    fn decide_literal_assignable(
        &self,
        literal: &dir::ScalarLiteral,
        target: &TypeLiteralTerm,
    ) -> Decision {
        match (literal, target) {
            (dir::ScalarLiteral::Null, TypeLiteralTerm::Null) => Decision::Yes,
            (dir::ScalarLiteral::Undefined, TypeLiteralTerm::Undefined) => Decision::Yes,
            (dir::ScalarLiteral::Boolean(_), target) if target == &TypeLiteralTerm::boolean() => {
                Decision::Yes
            }
            (
                dir::ScalarLiteral::Character(_),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Character),
            ) => Decision::Yes,
            (
                dir::ScalarLiteral::String(_),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::String),
            ) => Decision::Yes,
            (dir::ScalarLiteral::Bigint(_), target) if target == &TypeLiteralTerm::bigint() => {
                Decision::Yes
            }
            (
                dir::ScalarLiteral::Integer(value),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Integer(integer)),
            ) if Self::integer_literal_fits_integer(*value, *integer) => Decision::Yes,
            (
                dir::ScalarLiteral::Integer(value),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Float(float)),
            ) if Self::integer_literal_fits_float(*value, *float) => Decision::Yes,
            (
                dir::ScalarLiteral::Float(value),
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Float(float)),
            ) if Self::float_literal_fits_float(*value, *float) => Decision::Yes,
            (dir::ScalarLiteral::RegexString { .. }, TypeLiteralTerm::Object) => Decision::Yes,
            _ => Decision::No,
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
    fn decide_all_sources_assignable(
        &mut self,
        sources: &[TypeOperand],
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        for source in sources {
            let Some(source) = self.type_operand_term(*source)? else {
                return Ok(Decision::Undecidable);
            };
            let decision = self.decide_type_assignable(&source, target)?;
            if decision != Decision::Yes {
                return Ok(decision);
            }
        }

        Ok(Decision::Yes)
    }

    /// Decide whether one source assigns to any target union element.
    fn decide_any_target_assignable(
        &mut self,
        source: &TypeTerm,
        targets: &[TypeOperand],
    ) -> CompilerResult<Decision> {
        let mut has_unknown = false;

        // accept the first matching target
        for target in targets {
            let Some(target) = self.type_operand_term(*target)? else {
                has_unknown = true;
                continue;
            };
            match self.decide_type_assignable(source, &target)? {
                Decision::Yes => return Ok(Decision::Yes),
                Decision::Undecidable => has_unknown = true,
                Decision::No => {}
            }
        }

        if has_unknown {
            Ok(Decision::Undecidable)
        } else {
            Ok(Decision::No)
        }
    }
}
