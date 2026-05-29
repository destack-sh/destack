use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    AwaitTerm, CallCallee, CallTerm, CheckState, ConstructTerm, Decision, FormTerm, FunctionTerm,
    GenericArgument, GenericSlotId, GenericSubstitution, IdentityTerm, ImportMetaTerm,
    IndexSetTerm, IndexTerm, InstanceCheckTerm, KeyMembershipTerm, MemberCallCallee,
    MemberCallTerm, MemberProtocol, MemberTerm, OperatorTerm, Origin, Progress, RangeValueTerm,
    ReceiverTerm, Reduction, ShapeMember, StaticOperand, StaticRelation, StaticTerm, SuperTerm,
    TaggedTemplateTerm, TemplateTerm, TermId, TreeTerm, TryFailureTerm, TryTerm, TupleElement,
    TypeOperand, TypeOperationTerm, TypeRelation, TypeValueTerm, VariableId, VariableOutput,
    YieldTerm,
};

/// How an expected type flows into one defining term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeExpectationMode {
    /// The defining term must have exactly the solved result type.
    Exact,
    /// The defining term must be assignable to an upper bound.
    UpperBound,
}

/// Literal type value with no nested table references.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeLiteralTerm {
    /// Error type that could not be resolved.
    Error,
    /// Never type.
    Never,
    /// TypeScript `any` compatibility marker.
    Any,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Undefined type and value.
    Undefined,
    /// TypeScript object constraint.
    Object,
    /// Primitive type.
    Primitive(dir::PrimitiveType),
    /// Scalar literal type.
    Scalar(dir::ScalarLiteral),
    /// Compiler-provided type function.
    BuiltinTypeFunction(dir::BuiltinTypeFunction),
}

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
            dir::Type::Operation(dir::TypeOperation::BuiltinTypeFunction(function)) => {
                Self::BuiltinTypeFunction(*function)
            }
            _ => return None,
        };

        Some(literal)
    }

    /// Convert this literal into a committed DIR type.
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
            Self::BuiltinTypeFunction(function) => {
                dir::Type::Operation(dir::TypeOperation::BuiltinTypeFunction(*function))
            }
        }
    }
}

/// Term used to define a type variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeTerm {
    /// Literal concrete type.
    ///
    /// ```ts
    /// int32
    /// ```
    Literal(TypeLiteralTerm),
    /// Generic parameter reference.
    ///
    /// ```ts
    /// T
    /// ```
    Parameter(GenericSlotId),
    /// This type.
    ///
    /// ```ts
    /// this
    /// ```
    This,
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
    /// Static value projected into type position.
    ///
    /// ```ts
    /// type Tagged<comptime Tag: string> = { tag: Tag };
    /// ```
    StaticValue {
        /// The static value operand.
        value: StaticOperand,
    },
    /// Canonical memory form over a value type.
    ///
    /// ```ts
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
    /// Named type declaration reference.
    ///
    /// ```ts
    /// Map<K, V>
    /// ```
    Reference {
        /// The work origin that introduced this reference.
        origin: Origin,
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The applied static arguments.
        arguments: SmallVec<[GenericArgument; 4]>,
    },
    /// Homogeneous array type.
    Array {
        /// The element type.
        element: TypeOperand,
    },
    /// Type member projection.
    ///
    /// ```ts
    /// T.Item
    /// ```
    Member(TermId<MemberTerm>),
    /// Fixed-length array type.
    FixedArray {
        /// The repeated element type.
        element: TypeOperand,
        /// The static array length.
        length: StaticOperand,
        /// Whether the array is readonly.
        is_readonly: bool,
    },
    /// Runtime-length homogeneous view type.
    Slice {
        /// The element type.
        element: TypeOperand,
        /// Whether the slice is readonly.
        is_readonly: bool,
    },
    /// Tuple type.
    Tuple {
        /// The tuple source form.
        form: dir::TupleForm,
        /// The tuple elements.
        elements: SmallVec<[TupleElement; 4]>,
        /// Whether the tuple is readonly.
        is_readonly: bool,
    },
    /// Structural object shape type.
    Shape {
        /// The shape members.
        members: SmallVec<[ShapeMember; 8]>,
    },
    /// Function type.
    Function(TermId<FunctionTerm>),
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
        elements: Vec<TypeOperand>,
    },
    /// Intersection type.
    Intersection {
        /// The intersection elements.
        elements: Vec<TypeOperand>,
    },
    /// Type-level operation.
    ///
    /// ```ts
    /// keyof T
    /// ```
    Operation(TermId<TypeOperationTerm>),
    /// Runtime call expression.
    ///
    /// ```ts
    /// fn(value)
    /// ```
    Call(TermId<CallTerm>),
    /// Runtime construct expression.
    ///
    /// ```ts
    /// new User(value)
    /// ```
    Construct(TermId<ConstructTerm>),
    /// Runtime range value expression.
    ///
    /// ```ts
    /// start..end
    /// ```
    RangeValue(TermId<RangeValueTerm>),
    /// Runtime tree expression.
    ///
    /// ```tsx
    /// <Tag />
    /// ```
    Tree(TermId<TreeTerm>),
    /// Runtime reflected type value.
    ///
    /// ```ts
    /// type T
    /// ```
    TypeValue(TermId<TypeValueTerm>),
    /// Runtime import metadata value.
    ///
    /// ```ts
    /// import.meta
    /// ```
    ImportMeta(TermId<ImportMetaTerm>),
    /// Runtime contextual receiver.
    ///
    /// ```ts
    /// this
    /// ```
    Receiver(TermId<ReceiverTerm>),
    /// Runtime super receiver context.
    ///
    /// ```ts
    /// super
    /// ```
    Super(TermId<SuperTerm>),
    /// Runtime operator expression.
    ///
    /// ```ts
    /// -value
    /// left + right
    /// ```
    Operator(TermId<OperatorTerm>),
    /// Runtime index access.
    ///
    /// ```ts
    /// value[key]
    /// ```
    Index(TermId<IndexTerm>),
    /// Runtime index set.
    ///
    /// ```ts
    /// value[key] = next
    /// ```
    IndexSet(TermId<IndexSetTerm>),
    /// Runtime key membership check.
    ///
    /// ```ts
    /// "name" in value
    /// ```
    KeyMembership(TermId<KeyMembershipTerm>),
    /// Runtime nominal instance check.
    ///
    /// ```ts
    /// value instanceof Error
    /// ```
    InstanceCheck(TermId<InstanceCheckTerm>),
    /// Runtime identity equality check.
    ///
    /// ```ts
    /// left === right
    /// ```
    Identity(TermId<IdentityTerm>),
    /// Runtime await expression.
    ///
    /// ```ts
    /// await value
    /// ```
    Await(TermId<AwaitTerm>),
    /// Runtime try expression.
    ///
    /// ```ts
    /// value?
    /// ```
    Try(TermId<TryTerm>),
    /// Runtime yield expression.
    ///
    /// ```ts
    /// yield value
    /// ```
    Yield(TermId<YieldTerm>),
    /// Runtime try failure projection.
    ///
    /// ```ts
    /// try { value? } catch (error) { ... }
    /// ```
    TryFailure(TermId<TryFailureTerm>),
    /// Runtime template string expression.
    ///
    /// ```ts
    /// `hello ${name}`
    /// ```
    Template(TermId<TemplateTerm>),
    /// Runtime tagged template expression.
    ///
    /// ```ts
    /// sql`select ${id}`
    /// ```
    TaggedTemplate(TermId<TaggedTemplateTerm>),
    /// Type predicate.
    Predicate {
        /// Whether this is an assertion predicate.
        asserts: bool,
        /// The predicate subject.
        subject: dir::PredicateSubject,
        /// The predicate target type.
        target: Option<VariableId>,
    },
    /// Explicit runtime Dynamic type.
    ///
    /// ```ts
    /// Dynamic<T>
    /// ```
    Dynamic {
        /// The Dynamic constraint type.
        constraint: VariableId,
    },
    /// Closure type with its captured environment.
    ///
    /// ```ts
    /// () => value
    /// ```
    Closure {
        /// The function contract type.
        function: VariableId,
        /// The captured environment type.
        environment: VariableId,
    },
}

impl TypeTerm {
    /// Return the unit type.
    pub(in crate::check) fn unit() -> Self {
        TypeTerm::Tuple {
            form: dir::TupleForm::Tuple,
            elements: SmallVec::new(),
            is_readonly: false,
        }
    }

    /// Return whether this is the unit type.
    pub(in crate::check) fn is_unit(&self) -> bool {
        matches!(
            self,
            TypeTerm::Tuple {
                form: dir::TupleForm::Tuple,
                elements,
                is_readonly: false,
            } if elements.is_empty()
        )
    }

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Variable(variable) => variables.push(*variable),
            Self::StaticValue { value } => variables.extend(value.referenced_variables(state)),
            Self::Form { form, payload } => {
                variables.extend(payload.referenced_variables(state));
                variables.extend(state.terms.get(*form).referenced_variables(state));
            }
            Self::Reference {
                origin: _,
                symbol: _,
                arguments,
            } => {
                variables.extend(
                    arguments
                        .iter()
                        .flat_map(|argument| state.argument_variables(argument)),
                );
            }
            Self::Array { element } => variables.extend(element.referenced_variables(state)),
            Self::Member(member) => {
                variables.extend(state.terms.get(*member).referenced_variables(state));
            }
            Self::FixedArray {
                element,
                length,
                is_readonly: _,
            } => {
                variables.extend(element.referenced_variables(state));
                variables.extend(length.referenced_variables(state));
            }
            Self::Slice {
                element,
                is_readonly: _,
            } => variables.extend(element.referenced_variables(state)),
            Self::Tuple {
                form: _,
                elements,
                is_readonly: _,
            } => {
                for element in elements {
                    variables.extend(element.ty.referenced_variables(state));
                }
            }
            Self::Shape { members } => {
                for member in members {
                    variables.extend(member.referenced_variables(state));
                }
            }
            Self::Function(function) => {
                variables.extend(state.terms.get(*function).referenced_variables(state));
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
                variables.extend(state.terms.get(*operation).referenced_variables(state));
            }
            Self::Call(call) => {
                variables.extend(state.terms.get(*call).referenced_variables(state))
            }
            Self::Construct(construct) => {
                variables.extend(state.terms.get(*construct).referenced_variables(state));
            }
            Self::RangeValue(range) => {
                variables.extend(state.terms.get(*range).referenced_variables());
            }
            Self::Tree(tree) => {
                variables.extend(state.terms.get(*tree).referenced_variables(state))
            }
            Self::TypeValue(value) => {
                variables.extend(state.terms.get(*value).referenced_variables());
            }
            Self::ImportMeta(_) => {}
            Self::Receiver(receiver) => {
                variables.extend(state.terms.get(*receiver).referenced_variables());
            }
            Self::Super(term) => variables.extend(state.terms.get(*term).referenced_variables()),
            Self::Operator(operator) => {
                variables.extend(state.terms.get(*operator).referenced_variables());
            }
            Self::Index(index) => variables.extend(state.terms.get(*index).referenced_variables()),
            Self::IndexSet(set) => variables.extend(state.terms.get(*set).referenced_variables()),
            Self::KeyMembership(membership) => {
                variables.extend(state.terms.get(*membership).referenced_variables());
            }
            Self::InstanceCheck(instance) => {
                variables.extend(state.terms.get(*instance).referenced_variables());
            }
            Self::Identity(identity) => {
                variables.extend(state.terms.get(*identity).referenced_variables());
            }
            Self::Await(awaited) => {
                variables.extend(state.terms.get(*awaited).value.referenced_variables(state));
            }
            Self::Try(tried) => {
                variables.extend(state.terms.get(*tried).referenced_variables(state));
            }
            Self::Yield(yielded) => {
                variables.extend(state.terms.get(*yielded).referenced_variables())
            }
            Self::TryFailure(tried) => {
                variables.extend(state.terms.get(*tried).referenced_variables(state));
            }
            Self::Template(template) => {
                variables.extend(state.terms.get(*template).referenced_variables());
            }
            Self::TaggedTemplate(template) => {
                variables.extend(state.terms.get(*template).referenced_variables(state));
            }
            Self::Predicate {
                asserts: _,
                subject: _,
                target,
            } => variables.extend(target.iter().copied()),
            Self::Dynamic { constraint } => variables.push(*constraint),
            Self::Closure {
                function,
                environment,
            } => {
                variables.push(*function);
                variables.push(*environment);
            }
            Self::Literal(_)
            | Self::Parameter(_)
            | Self::This
            | Self::Intrinsic
            | Self::ConstAssertion => {}
        }

        variables
    }

    /// Return whether this term must reduce before it can be a stable solution.
    pub(in crate::check) fn is_pending_reduction(&self) -> bool {
        matches!(
            self,
            Self::Reference { .. }
                | Self::Member { .. }
                | Self::StaticValue { .. }
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
                | Self::TaggedTemplate(_)
        )
    }
}

impl CheckState<'_> {
    /// Constrain one literal expression variable by its contextual type.
    pub(in crate::check) fn expect_literal_term(
        &mut self,
        variable: VariableId,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(_)) = source else {
            return Ok(Progress::Unchanged);
        };
        if self.decide_type_term_relation(TypeRelation::Assignable, source, target)?
            != Decision::Yes
        {
            return Ok(Progress::Unchanged);
        }
        if !self.variable_is_scalar_literal_node(variable) {
            return Ok(Progress::Unchanged);
        }
        let target = self.terms.push(target.clone());

        self.solve_type_assignability(self.variable(variable).source, variable, target)
    }

    /// Return whether one variable is backed by a scalar literal expression node.
    fn variable_is_scalar_literal_node(&self, variable: VariableId) -> bool {
        let Some(VariableOutput::Node(node)) = self.variable(variable).output else {
            return false;
        };
        if node.local_id.ty != dir::NodeType::Expression {
            return false;
        }
        let view = self.module(variable.module).view();
        let expression = dir::LocalNodeId::<dir::Expression>::new(node.local_id.id);

        matches!(
            view.tree().get(expression),
            dir::Expression::ScalarLiteral(_)
        )
    }

    /// Return the element count when one variable is backed by an array literal.
    fn variable_array_literal_length(&self, variable: VariableId) -> Option<usize> {
        let Some(VariableOutput::Node(node)) = self.variable(variable).output else {
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

    /// Constrain one array literal expression variable by its contextual type.
    fn expect_array_literal_term(
        &mut self,
        variable: VariableId,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        if self.variable_array_literal_length(variable).is_none() {
            return Ok(Progress::Unchanged);
        }
        let target = self.terms.push(target.clone());

        self.solve_type_assignability(self.variable(variable).source, variable, target)
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
        let length = self
            .terms
            .push(StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::Integer(length as i64),
            }));
        let length = StaticOperand::Term(length);
        if self.decide_static_relation(StaticRelation::Equal, length, target)? == Decision::No {
            return Ok(None);
        }

        let progress = self.solve_static_equality(length, target)?;
        let decision = self.decide_static_relation(StaticRelation::Equal, length, target)?;
        if decision == Decision::Yes {
            Ok(Some(progress))
        } else {
            Ok(None)
        }
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
            let literal = self.expect_literal_term(variable, &term, expected_term)?;
            let definition = self.expect_type_definition(
                origin,
                Some(variable),
                expected,
                expected_term,
                &term,
                TypeExpectationMode::UpperBound,
            )?;

            return Ok(literal.merge(definition));
        }

        self.expect_type_definition(
            origin,
            None,
            expected,
            expected_term,
            &term,
            TypeExpectationMode::UpperBound,
        )
    }

    /// Decide one type term relation.
    pub(in crate::check) fn decide_type_term_relation(
        &self,
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
        &self,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) -> CompilerResult<Decision> {
        let left = match left.into() {
            TypeOperand::Variable(variable) => {
                let Some(term) = self.solved_type_term(variable)? else {
                    return Ok(Decision::Undecidable);
                };

                term
            }
            TypeOperand::Term(term) => self.terms.get(term).clone(),
        };
        let right = match right.into() {
            TypeOperand::Variable(variable) => {
                let Some(term) = self.solved_type_term(variable)? else {
                    return Ok(Decision::Undecidable);
                };

                term
            }
            TypeOperand::Term(term) => self.terms.get(term).clone(),
        };

        self.decide_type_term_relation(relation, &left, &right)
    }

    /// Expect the defining term to satisfy the result type.
    pub(in crate::check) fn expect_type_term(
        &mut self,
        origin: Origin,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push exact external result when it is known
        let Some(result_term) = self.solved_type_term(result)? else {
            return Ok(Progress::Unchanged);
        };
        if &result_term != term {
            progress = progress.merge(self.expect_type_definition(
                origin,
                Some(result),
                TypeOperand::Variable(result),
                &result_term,
                term,
                TypeExpectationMode::Exact,
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
            TypeTerm::Variable(variable) => match self.solved_type_term(*variable)? {
                Some(term) => Reduction::value(term),
                None => Reduction::pending(),
            },
            TypeTerm::Member(member) => {
                let member = self.terms.get(*member).clone();

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
            TypeTerm::Operation(operation) => match self.terms.get(*operation).clone() {
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
                let call = self.terms.get(*call).clone();

                self.reduce_call_term(origin, module, &call)?
            }
            TypeTerm::Construct(construct) => {
                let construct = self.terms.get(*construct).clone();

                self.reduce_construct_term(origin, module, &construct)?
            }
            TypeTerm::RangeValue(range) => {
                let range = self.terms.get(*range).clone();

                self.reduce_range_value_term(module, &range)?
            }
            TypeTerm::Tree(tree) => {
                let tree = self.terms.get(*tree).clone();

                self.reduce_tree_term(module, &tree)?
            }
            TypeTerm::TypeValue(value) => {
                let value = self.terms.get(*value).clone();

                self.reduce_type_value_term(module, &value)?
            }
            TypeTerm::ImportMeta(meta) => {
                let meta = self.terms.get(*meta).clone();

                self.reduce_import_meta_term(module, &meta)?
            }
            TypeTerm::Receiver(receiver) => {
                let receiver = self.terms.get(*receiver).clone();

                self.reduce_receiver_term(&receiver)?
            }
            TypeTerm::Super(term) => {
                let term = self.terms.get(*term).clone();

                self.reduce_super_term(module, &term)?
            }
            TypeTerm::Operator(operator) => {
                let operator = self.terms.get(*operator).clone();

                match self.reduce_operator_term(origin, &operator)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Index(index) => {
                let index = self.terms.get(*index).clone();

                self.reduce_index_term(origin, module, &index)?
            }
            TypeTerm::IndexSet(set) => {
                let set = self.terms.get(*set).clone();

                self.reduce_index_set_term(origin, module, &set)?
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = self.terms.get(*membership).clone();

                match self.reduce_key_membership_term(&membership)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = self.terms.get(*instance).clone();

                match self.reduce_instance_check_term(&instance)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Identity(identity) => {
                let identity = self.terms.get(*identity).clone();

                match self.reduce_identity_term(&identity)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Template(template) => {
                let template = self.terms.get(*template).clone();

                match self.reduce_template_term(&template)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::TaggedTemplate(template) => {
                let template = self.terms.get(*template).clone();

                self.reduce_tagged_template_term(origin, &template)?
            }
            TypeTerm::Await(awaited) => {
                let awaited = self.terms.get(*awaited).clone();

                match self.reduce_await_term(&awaited)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Try(tried) => {
                let tried = self.terms.get(*tried).clone();

                match self.reduce_try_term(origin, module, &tried)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.terms.get(*yielded).clone();

                match self.reduce_yield_term(&yielded)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::TryFailure(tried) => {
                let tried = self.terms.get(*tried).clone();

                match self.reduce_try_failure_term(origin, module, &tried)? {
                    Some(term) => Reduction::value(term),
                    None => Reduction::pending(),
                }
            }
            TypeTerm::Form { form, payload } => {
                let payload = self.reduce_type_operand_to_operand(origin, *payload)?;

                Reduction::value(TypeTerm::Form {
                    form: *form,
                    payload,
                })
            }
            TypeTerm::Array { element } => {
                let element = self.reduce_type_operand_to_operand(origin, *element)?;

                Reduction::value(TypeTerm::Array { element })
            }
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => {
                let element = self.reduce_type_operand_to_operand(origin, *element)?;

                Reduction::value(TypeTerm::FixedArray {
                    element,
                    length: *length,
                    is_readonly: *is_readonly,
                })
            }
            TypeTerm::Slice {
                element,
                is_readonly,
            } => {
                let element = self.reduce_type_operand_to_operand(origin, *element)?;

                Reduction::value(TypeTerm::Slice {
                    element,
                    is_readonly: *is_readonly,
                })
            }
            TypeTerm::Tuple {
                form,
                elements,
                is_readonly,
            } => {
                let elements = self.reduce_tuple_term(origin, elements)?;

                Reduction::value(TypeTerm::Tuple {
                    form: *form,
                    elements,
                    is_readonly: *is_readonly,
                })
            }
            TypeTerm::Union { elements } => {
                let elements = self.reduce_type_operands_to_operands(origin, elements)?;

                Reduction::value(TypeTerm::Union { elements })
            }
            TypeTerm::Intersection { elements } => {
                let elements = self.reduce_type_operands_to_operands(origin, elements)?;

                Reduction::value(TypeTerm::Intersection { elements })
            }
            TypeTerm::Literal(_)
            | TypeTerm::Parameter(_)
            | TypeTerm::This
            | TypeTerm::Intrinsic
            | TypeTerm::ConstAssertion
            | TypeTerm::Function(_)
            | TypeTerm::Range { .. }
            | TypeTerm::Predicate { .. }
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Closure { .. } => Reduction::value(term.clone()),
            TypeTerm::Shape { members } => {
                let members = self.reduce_shape_term(origin, members)?;

                Reduction::value(TypeTerm::Shape { members })
            }
        };

        Ok(reduction)
    }

    /// Reduce nested type operands.
    fn reduce_type_operands_to_operands(
        &mut self,
        origin: Origin,
        operands: &[TypeOperand],
    ) -> CompilerResult<Vec<TypeOperand>> {
        let mut reduced = Vec::with_capacity(operands.len());

        // reduce operands in source order
        for operand in operands {
            reduced.push(self.reduce_type_operand_to_operand(origin, *operand)?);
        }

        Ok(reduced)
    }

    /// Reduce nested operands inside one tuple.
    fn reduce_tuple_term(
        &mut self,
        origin: Origin,
        elements: &[TupleElement],
    ) -> CompilerResult<SmallVec<[TupleElement; 4]>> {
        let mut reduced = SmallVec::with_capacity(elements.len());

        // reduce elements in source order
        for element in elements {
            let ty = self.reduce_type_operand_to_operand(origin, element.ty)?;

            reduced.push(TupleElement { ty, ..*element });
        }

        Ok(reduced)
    }

    /// Reduce nested operands inside one structural shape.
    fn reduce_shape_term(
        &mut self,
        origin: Origin,
        members: &[ShapeMember],
    ) -> CompilerResult<SmallVec<[ShapeMember; 8]>> {
        let mut reduced = SmallVec::with_capacity(members.len());

        // reduce members in source order
        for member in members {
            reduced.push(self.reduce_shape_member(origin, member)?);
        }

        Ok(reduced)
    }

    /// Reduce nested operands inside one shape member.
    fn reduce_shape_member(
        &mut self,
        origin: Origin,
        member: &ShapeMember,
    ) -> CompilerResult<ShapeMember> {
        let member = match member {
            ShapeMember::Field {
                key,
                ty,
                is_optional,
                is_readonly,
            } => ShapeMember::Field {
                key: *key,
                ty: self.reduce_type_operand_to_operand(origin, *ty)?,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
            ShapeMember::CallSignature { ty } => ShapeMember::CallSignature {
                ty: self.reduce_type_operand_to_operand(origin, *ty)?,
            },
            ShapeMember::ConstructSignature { ty } => ShapeMember::ConstructSignature {
                ty: self.reduce_type_operand_to_operand(origin, *ty)?,
            },
            ShapeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => ShapeMember::IndexSignature {
                name: *name,
                key_type: self.reduce_type_operand_to_operand(origin, *key_type)?,
                value_type: self.reduce_type_operand_to_operand(origin, *value_type)?,
                is_optional: *is_optional,
                is_readonly: *is_readonly,
            },
        };

        Ok(member)
    }

    /// Reduce one operand while preserving the original operand when unchanged.
    fn reduce_type_operand_to_operand(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let Some(original) = self.type_operand_term(operand)? else {
            return Ok(operand);
        };
        let Some(reduced) = self.reduce_type_operand_term(origin, original.clone())? else {
            return Ok(operand);
        };
        if reduced == original {
            return Ok(operand);
        }
        if matches!(operand, TypeOperand::Variable(_)) {
            return Ok(operand);
        }
        let reduced = self.terms.push(reduced);

        Ok(reduced.into())
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
                let left_form = self.terms.get(*left_form).clone();
                let right_form = self.terms.get(*right_form).clone();
                let form = self.constrain_form_equal(&left_form, &right_form)?;
                let value = self.solve_type_equality(origin, *left_value, *right_value)?;

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
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: right_element,
                    is_readonly: _,
                },
            ) => self.solve_type_equality(origin, *left_element, *right_element)?,
            (
                TypeTerm::FixedArray {
                    element: left_element,
                    length: left_length,
                    is_readonly: _,
                },
                TypeTerm::FixedArray {
                    element: right_element,
                    length: right_length,
                    is_readonly: _,
                },
            ) => {
                let element = self.solve_type_equality(origin, *left_element, *right_element)?;
                let length = self.solve_static_equality(*left_length, *right_length)?;

                element.merge(length)
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left,
                    is_readonly: _,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right,
                    is_readonly: _,
                },
            ) => {
                if left_form == right_form {
                    self.constrain_tuple_elements_equal(origin, left, right)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape { members: left }, TypeTerm::Shape { members: right }) => {
                self.constrain_shape_members_equal(origin, left, right)?
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
                let source_form = self.terms.get(*source_form).clone();
                let target_form = self.terms.get(*target_form).clone();
                let form = self.constrain_form_assignable(&source_form, &target_form)?;
                let value = self.solve_type_assignability(origin, *source_value, *target_value)?;

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
                    is_readonly: _,
                },
            )
            | (
                TypeTerm::Slice {
                    element: source_element,
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: target_element,
                    is_readonly: _,
                },
            ) => self.solve_type_assignability(origin, *source_element, *target_element)?,
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: _,
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: target_element,
                    is_readonly: _,
                },
            ) => self.solve_type_assignability(origin, *source_element, *target_element)?,
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: source_length,
                    is_readonly: _,
                },
                TypeTerm::FixedArray {
                    element: target_element,
                    length: target_length,
                    is_readonly: _,
                },
            ) => {
                let element =
                    self.solve_type_assignability(origin, *source_element, *target_element)?;
                let length = self.solve_static_equality(*source_length, *target_length)?;

                element.merge(length)
            }
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: source,
                    is_readonly: _,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: target,
                    is_readonly: _,
                },
            ) => {
                if source_form == target_form {
                    self.constrain_tuple_elements_assignable(origin, source, target)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: target }) => {
                self.constrain_shape_members_assignable(origin, source, target)?
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
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match self {
            TypeTerm::Variable(variable) => {
                if let Some(argument) = state.substitution_type_variable(substitution, *variable) {
                    TypeTerm::Variable(argument)
                } else if let Some(term) = state.solved_type_term(*variable)? {
                    if let Some(term) = term.substitute(module, substitution, state)? {
                        term
                    } else {
                        TypeTerm::Variable(*variable)
                    }
                } else {
                    TypeTerm::Variable(*variable)
                }
            }
            TypeTerm::Parameter(slot_id) => {
                if let Some(argument) = state.substitution_type_slot(substitution, *slot_id) {
                    TypeTerm::Variable(argument)
                } else if let Some(argument) =
                    state.substitution_static_slot(substitution, *slot_id)
                {
                    TypeTerm::StaticValue {
                        value: argument.into(),
                    }
                } else {
                    self.clone()
                }
            }
            TypeTerm::StaticValue { value } => TypeTerm::StaticValue {
                value: state.substitute_static_operand(module, substitution, *value)?,
            },
            TypeTerm::Form { form, payload } => TypeTerm::Form {
                form: {
                    let form = state.terms.get(*form).clone();
                    let form = form.substitute(module, substitution, state)?;
                    state.terms.push(form)
                },
                payload: state.substitute_type_operand(module, substitution, *payload)?,
            },
            TypeTerm::Reference {
                origin: reference_origin,
                symbol,
                arguments,
            } => {
                if arguments.is_empty()
                    && let Some(argument) = state.substitution_type_symbol(substitution, *symbol)
                {
                    TypeTerm::Variable(argument)
                } else {
                    TypeTerm::Reference {
                        origin: *reference_origin,
                        symbol: *symbol,
                        arguments: state.substitute_arguments(module, substitution, arguments)?,
                    }
                }
            }
            TypeTerm::Array { element } => TypeTerm::Array {
                element: state.substitute_type_operand(module, substitution, *element)?,
            },
            TypeTerm::Member(member) => {
                let member = state.terms.get(*member).clone();
                let member = member.substitute(module, substitution, state)?;
                let member = state.terms.push(member);

                TypeTerm::Member(member)
            }
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => TypeTerm::FixedArray {
                element: state.substitute_type_operand(module, substitution, *element)?,
                length: state.substitute_static_operand(module, substitution, *length)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Slice {
                element,
                is_readonly,
            } => TypeTerm::Slice {
                element: state.substitute_type_operand(module, substitution, *element)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Tuple {
                form,
                elements,
                is_readonly,
            } => TypeTerm::Tuple {
                form: *form,
                elements: state
                    .substitute_tuple_elements(module, substitution, elements)?
                    .into(),
                is_readonly: *is_readonly,
            },
            TypeTerm::Shape { members } => TypeTerm::Shape {
                members: state
                    .substitute_shape_members(module, substitution, members)?
                    .into(),
            },
            TypeTerm::Function(function) => {
                let function = state.terms.get(*function).clone();
                let function = function.substitute(module, substitution, state)?;
                let function = state.terms.push(function);

                TypeTerm::Function(function)
            }
            TypeTerm::Union { elements } => TypeTerm::Union {
                elements: state.substitute_type_operands(module, substitution, elements)?,
            },
            TypeTerm::Intersection { elements } => TypeTerm::Intersection {
                elements: state.substitute_type_operands(module, substitution, elements)?,
            },
            TypeTerm::Operation(operation) => {
                let operation = state.terms.get(*operation).clone();
                let operation = operation.substitute(module, substitution, state)?;
                let operation = state.terms.push(operation);

                TypeTerm::Operation(operation)
            }
            TypeTerm::Call(call) => {
                let call = state.terms.get(*call).clone();
                let callee = match call.callee {
                    CallCallee::Value(callee) => CallCallee::Value(
                        state.substitute_type_variable(module, substitution, callee)?,
                    ),
                    CallCallee::Member(member) => {
                        let member = state.terms.get(member).clone();
                        let callee = match member.callee {
                            MemberCallCallee::Source { source } => {
                                MemberCallCallee::Source { source }
                            }
                            MemberCallCallee::Protocol { protocol } => MemberCallCallee::Protocol {
                                protocol: MemberProtocol {
                                    item: protocol.item,
                                    arguments: state.substitute_arguments(
                                        module,
                                        substitution,
                                        &protocol.arguments,
                                    )?,
                                },
                            },
                        };
                        let member = MemberCallTerm {
                            callee,
                            receiver: state.substitute_type_variable(
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

                        CallCallee::Member(state.terms.push(member))
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
                };

                TypeTerm::Call(state.terms.push(call))
            }
            TypeTerm::Construct(construct) => {
                let construct = state.terms.get(*construct).clone();
                let construct = ConstructTerm {
                    source: construct.source,
                    callee: state.substitute_type_variable(
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
                let construct = state.terms.push(construct);

                TypeTerm::Construct(construct)
            }
            TypeTerm::RangeValue(range) => {
                let range = state.terms.get(*range).clone();
                let range = RangeValueTerm {
                    source: range.source,
                    start: range
                        .start
                        .map(|start| state.substitute_type_variable(module, substitution, start))
                        .transpose()?,
                    end: range
                        .end
                        .map(|end| state.substitute_type_variable(module, substitution, end))
                        .transpose()?,
                    end_kind: range.end_kind,
                };
                let range = state.terms.push(range);

                TypeTerm::RangeValue(range)
            }
            TypeTerm::Tree(tree) => {
                let tree = state.terms.get(*tree).clone();
                let tree = TreeTerm {
                    source: tree.source,
                    tag: tree
                        .tag
                        .map(|tag| state.substitute_type_variable(module, substitution, tag))
                        .transpose()?,
                    generic_arguments: state.substitute_arguments(
                        module,
                        substitution,
                        &tree.generic_arguments,
                    )?,
                    arguments: state.substitute_type_variables(
                        module,
                        substitution,
                        &tree.arguments,
                    )?,
                    elements: state.substitute_type_variables(
                        module,
                        substitution,
                        &tree.elements,
                    )?,
                };
                let tree = state.terms.push(tree);

                TypeTerm::Tree(tree)
            }
            TypeTerm::TypeValue(value) => {
                let value = state.terms.get(*value).clone();
                let value = TypeValueTerm {
                    source: value.source,
                    ty: state.substitute_type_variable(module, substitution, value.ty)?,
                };
                let value = state.terms.push(value);

                TypeTerm::TypeValue(value)
            }
            TypeTerm::ImportMeta(meta) => TypeTerm::ImportMeta(*meta),
            TypeTerm::Receiver(receiver) => {
                let receiver = state.terms.get(*receiver).clone();
                let receiver = ReceiverTerm {
                    source: receiver.source,
                    kind: receiver.kind,
                    ty: state.substitute_type_variable(module, substitution, receiver.ty)?,
                };
                let receiver = state.terms.push(receiver);

                TypeTerm::Receiver(receiver)
            }
            TypeTerm::Super(term) => {
                let term = state.terms.get(*term).clone();
                let term = SuperTerm {
                    source: term.source,
                    receiver: term
                        .receiver
                        .map(|receiver| {
                            state.substitute_type_variable(module, substitution, receiver)
                        })
                        .transpose()?,
                };
                let term = state.terms.push(term);

                TypeTerm::Super(term)
            }
            TypeTerm::Operator(operator) => {
                let operator = state.terms.get(*operator).clone();
                let operator = OperatorTerm {
                    source: operator.source,
                    kind: operator.kind,
                    receiver: state.substitute_type_variable(
                        module,
                        substitution,
                        operator.receiver,
                    )?,
                    argument: operator
                        .argument
                        .map(|argument| {
                            state.substitute_type_variable(module, substitution, argument)
                        })
                        .transpose()?,
                };
                let operator = state.terms.push(operator);

                TypeTerm::Operator(operator)
            }
            TypeTerm::Index(index) => {
                let index = state.terms.get(*index).clone();
                let index = IndexTerm {
                    source: index.source,
                    receiver: state.substitute_type_variable(
                        module,
                        substitution,
                        index.receiver,
                    )?,
                    index: state.substitute_type_variable(module, substitution, index.index)?,
                    key: index.key,
                };
                let index = state.terms.push(index);

                TypeTerm::Index(index)
            }
            TypeTerm::IndexSet(set) => {
                let set = state.terms.get(*set).clone();
                let set = IndexSetTerm {
                    source: set.source,
                    receiver: state.substitute_type_variable(module, substitution, set.receiver)?,
                    index: state.substitute_type_variable(module, substitution, set.index)?,
                    value: state.substitute_type_variable(module, substitution, set.value)?,
                    key: set.key,
                };
                let set = state.terms.push(set);

                TypeTerm::IndexSet(set)
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = state.terms.get(*membership).clone();
                let membership = KeyMembershipTerm {
                    source: membership.source,
                    key: state.substitute_type_variable(module, substitution, membership.key)?,
                    receiver: state.substitute_type_variable(
                        module,
                        substitution,
                        membership.receiver,
                    )?,
                    static_key: membership.static_key,
                };
                let membership = state.terms.push(membership);

                TypeTerm::KeyMembership(membership)
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = state.terms.get(*instance).clone();
                let instance = InstanceCheckTerm {
                    source: instance.source,
                    value: state.substitute_type_variable(module, substitution, instance.value)?,
                    target: state.substitute_type_variable(
                        module,
                        substitution,
                        instance.target,
                    )?,
                };
                let instance = state.terms.push(instance);

                TypeTerm::InstanceCheck(instance)
            }
            TypeTerm::Identity(identity) => {
                let identity = state.terms.get(*identity).clone();
                let identity = IdentityTerm {
                    source: identity.source,
                    operator: identity.operator,
                    left: state.substitute_type_variable(module, substitution, identity.left)?,
                    right: state.substitute_type_variable(module, substitution, identity.right)?,
                };
                let identity = state.terms.push(identity);

                TypeTerm::Identity(identity)
            }
            TypeTerm::Await(awaited) => {
                let awaited = state.terms.get(*awaited).clone();
                let awaited = AwaitTerm {
                    source: awaited.source,
                    value: state.substitute_type_operand(module, substitution, awaited.value)?,
                };
                let awaited = state.terms.push(awaited);

                TypeTerm::Await(awaited)
            }
            TypeTerm::Try(tried) => {
                let tried = state.terms.get(*tried).clone();
                let tried = TryTerm {
                    source: tried.source,
                    value: state.substitute_type_operand(module, substitution, tried.value)?,
                    kind: tried.kind,
                };
                let tried = state.terms.push(tried);

                TypeTerm::Try(tried)
            }
            TypeTerm::Yield(yielded) => {
                let yielded = state.terms.get(*yielded).clone();
                let yielded = YieldTerm {
                    source: yielded.source,
                    value: yielded
                        .value
                        .map(|value| state.substitute_type_variable(module, substitution, value))
                        .transpose()?,
                    yield_type: yielded
                        .yield_type
                        .map(|ty| state.substitute_type_variable(module, substitution, ty))
                        .transpose()?,
                    resume_type: yielded
                        .resume_type
                        .map(|ty| state.substitute_type_variable(module, substitution, ty))
                        .transpose()?,
                    delegate_return_type: yielded
                        .delegate_return_type
                        .map(|ty| state.substitute_type_variable(module, substitution, ty))
                        .transpose()?,
                    cardinality: yielded.cardinality,
                };
                let yielded = state.terms.push(yielded);

                TypeTerm::Yield(yielded)
            }
            TypeTerm::TryFailure(tried) => {
                let tried = state.terms.get(*tried).clone();
                let tried = TryFailureTerm {
                    source: tried.source,
                    value: state.substitute_type_operand(module, substitution, tried.value)?,
                };
                let tried = state.terms.push(tried);

                TypeTerm::TryFailure(tried)
            }
            TypeTerm::Template(template) => {
                let template = state.terms.get(*template).clone();
                let template = TemplateTerm {
                    source: template.source,
                    strings: template.strings.clone(),
                    spans: state.substitute_type_variables(
                        module,
                        substitution,
                        &template.spans,
                    )?,
                };
                let template = state.terms.push(template);

                TypeTerm::Template(template)
            }
            TypeTerm::TaggedTemplate(template) => {
                let template = state.terms.get(*template).clone();
                let template = TaggedTemplateTerm {
                    source: template.source,
                    tag: state.substitute_type_variable(module, substitution, template.tag)?,
                    generic_arguments: state.substitute_arguments(
                        module,
                        substitution,
                        &template.generic_arguments,
                    )?,
                    strings: template.strings.clone(),
                    spans: state.substitute_type_variables(
                        module,
                        substitution,
                        &template.spans,
                    )?,
                };
                let template = state.terms.push(template);

                TypeTerm::TaggedTemplate(template)
            }
            TypeTerm::Predicate {
                asserts,
                subject,
                target,
            } => TypeTerm::Predicate {
                asserts: *asserts,
                subject: *subject,
                target: target
                    .map(|target| state.substitute_type_variable(module, substitution, target))
                    .transpose()?,
            },
            TypeTerm::Dynamic { constraint } => TypeTerm::Dynamic {
                constraint: state.substitute_type_variable(module, substitution, *constraint)?,
            },
            TypeTerm::Closure {
                function,
                environment,
            } => TypeTerm::Closure {
                function: state.substitute_type_variable(module, substitution, *function)?,
                environment: state.substitute_type_variable(module, substitution, *environment)?,
            },
            TypeTerm::Literal(_)
            | TypeTerm::This
            | TypeTerm::Intrinsic
            | TypeTerm::ConstAssertion
            | TypeTerm::Range { .. } => self.clone(),
        };

        Ok(Some(term))
    }
}

impl CheckState<'_> {
    /// Decide exact equality for type variable lists.
    pub(in crate::check) fn decide_type_variable_list_equal<L, R>(
        &self,
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
        &self,
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
        mode: TypeExpectationMode,
    ) -> CompilerResult<Progress> {
        let progress = match term {
            TypeTerm::Variable(variable) => match mode {
                TypeExpectationMode::Exact => {
                    self.solve_type_equality(origin, *variable, expected)?
                }
                TypeExpectationMode::UpperBound => Progress::Unchanged,
            },
            TypeTerm::Literal(_) => match result {
                Some(result) => self.expect_literal_term(result, term, expected_term)?,
                None => Progress::Unchanged,
            },
            TypeTerm::Form { form, payload } => {
                let TypeTerm::Form {
                    form: result_form,
                    payload: result_payload,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let form_term = self.terms.get(*form).clone();
                let result_form = self.terms.get(*result_form).clone();
                let form = self.expect_form_term(&form_term, &result_form)?;
                let payload =
                    self.solve_contextual_type_assignability(origin, *payload, *result_payload)?;

                form.merge(payload)
            }
            TypeTerm::Array { element } => match expected_term {
                TypeTerm::Array {
                    element: result_element,
                }
                | TypeTerm::Slice {
                    element: result_element,
                    is_readonly: _,
                } => {
                    let element = self.solve_contextual_type_assignability(
                        origin,
                        *element,
                        *result_element,
                    )?;
                    let literal = result
                        .map(|result| self.expect_array_literal_term(result, expected_term))
                        .transpose()?
                        .unwrap_or(Progress::Unchanged);

                    element.merge(literal)
                }
                TypeTerm::FixedArray {
                    element: result_element,
                    length: result_length,
                    is_readonly: _,
                } => {
                    let Some(result) = result else {
                        return Ok(Progress::Unchanged);
                    };
                    let Some(length) =
                        self.expect_array_literal_length(result, (*result_length).into())?
                    else {
                        return Ok(Progress::Unchanged);
                    };
                    let element = self.solve_contextual_type_assignability(
                        origin,
                        *element,
                        *result_element,
                    )?;
                    let literal = self.expect_array_literal_term(result, expected_term)?;

                    length.merge(element).merge(literal)
                }
                _ => Progress::Unchanged,
            },
            TypeTerm::Slice {
                element,
                is_readonly: _,
            } => {
                let TypeTerm::Slice {
                    element: result_element,
                    is_readonly: _,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.solve_contextual_type_assignability(origin, *element, *result_element)?
            }
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly: _,
            } => {
                let TypeTerm::FixedArray {
                    element: result_element,
                    length: result_length,
                    is_readonly: _,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let element =
                    self.solve_contextual_type_assignability(origin, *element, *result_element)?;
                let length = self.solve_static_equality(*length, *result_length)?;

                element.merge(length)
            }
            TypeTerm::Tuple {
                form: _,
                elements,
                is_readonly: _,
            } => {
                let TypeTerm::Tuple {
                    form: _,
                    elements: result_elements,
                    is_readonly: _,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_tuple_element_terms(origin, elements, &result_elements)?
            }
            TypeTerm::Shape { members } => {
                let TypeTerm::Shape {
                    members: result_members,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_shape_member_terms(origin, members, &result_members)?
            }
            TypeTerm::Call(call) => {
                let call = self.terms.get(*call).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_call_term(origin, &call, expected)?
            }
            TypeTerm::Construct(construct) => {
                let construct = self.terms.get(*construct).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_construct_term(origin, &construct, expected)?
            }
            TypeTerm::Operator(operator) => {
                let operator = self.terms.get(*operator).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_operator_term(origin, &operator, expected)?
            }
            TypeTerm::Index(index) => {
                let index = self.terms.get(*index).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_index_term(origin, &index, expected)?
            }
            TypeTerm::IndexSet(set) => {
                let set = self.terms.get(*set).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_index_set_term(origin, &set, expected)?
            }
            TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_) => Progress::Unchanged,
            TypeTerm::TaggedTemplate(template) => {
                let template = self.terms.get(*template).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_tagged_template_term(origin, &template, expected)?
            }
            TypeTerm::Await(awaited) => {
                let awaited = self.terms.get(*awaited).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_await_term(origin, &awaited, expected)?
            }
            TypeTerm::Try(tried) => {
                let tried = self.terms.get(*tried).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_try_term(origin, &tried, expected)?
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.terms.get(*yielded).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_yield_term(origin, &yielded, expected)?
            }
            TypeTerm::TryFailure(tried) => {
                let tried = self.terms.get(*tried).clone();
                let Some(expected) = expected.variable() else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_try_failure_term(origin, &tried, expected)?
            }
            TypeTerm::Operation(operation) => match self.terms.get(*operation).clone() {
                TypeOperationTerm::Exclude { source, target: _ } => {
                    self.solve_contextual_type_assignability(origin, source, expected)?
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
                } => self.expect_conditional_term(
                    origin,
                    left,
                    right,
                    then_type,
                    else_type,
                    expected,
                    mode == TypeExpectationMode::Exact,
                )?,
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
                    mode == TypeExpectationMode::Exact,
                )?,
                TypeOperationTerm::TemplateLiteral { .. }
                | TypeOperationTerm::Infer { .. }
                | TypeOperationTerm::KeyOf { .. }
                | TypeOperationTerm::Mapped { .. }
                | TypeOperationTerm::Intrinsic { .. } => Progress::Unchanged,
            },
            TypeTerm::RangeValue(range) => {
                let range = self.terms.get(*range).clone();

                self.expect_range_value_term(origin, &range, expected_term)?
            }
            TypeTerm::Function(function) => {
                let TypeTerm::Function(expected_function) = expected_term else {
                    return Ok(Progress::Unchanged);
                };

                let function = self.terms.get(*function).clone();
                let expected_function = self.terms.get(*expected_function).clone();

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
            | TypeTerm::Predicate { .. }
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Closure { .. }
            | TypeTerm::Parameter(_)
            | TypeTerm::This
            | TypeTerm::Intrinsic
            | TypeTerm::ConstAssertion => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decide whether an explicit cast is valid.
    fn decide_type_castable(
        &self,
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
    fn decide_type_equal(&self, left: &TypeTerm, right: &TypeTerm) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), right) if right.is_unit() => Decision::Yes,
            (left, TypeTerm::Literal(TypeLiteralTerm::Void)) if left.is_unit() => Decision::Yes,
            (TypeTerm::Variable(left), right) => {
                let Some(left) = self.solved_type_term(*left)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_equal(&left, right)?
            }
            (left, TypeTerm::Variable(right)) => {
                let Some(right) = self.solved_type_term(*right)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_equal(left, &right)?
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
                let form = self
                    .decide_form_equal(self.terms.get(*left_form), self.terms.get(*right_form))?;
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
                    is_readonly: left_readonly,
                },
                TypeTerm::Slice {
                    element: right_element,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_readonly != right_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Equal, *left_element, *right_element)?
                }
            }
            (
                TypeTerm::FixedArray {
                    element: left_element,
                    length: left_length,
                    is_readonly: left_readonly,
                },
                TypeTerm::FixedArray {
                    element: right_element,
                    length: right_length,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_readonly != right_readonly {
                    Decision::No
                } else {
                    let element = self.decide_type_relation(
                        TypeRelation::Equal,
                        *left_element,
                        *right_element,
                    )?;
                    let length = self.decide_static_relation(
                        StaticRelation::Equal,
                        *left_length,
                        *right_length,
                    )?;

                    element.and(length)
                }
            }
            (
                TypeTerm::Tuple {
                    form: left_form,
                    elements: left_elements,
                    is_readonly: left_readonly,
                },
                TypeTerm::Tuple {
                    form: right_form,
                    elements: right_elements,
                    is_readonly: right_readonly,
                },
            ) => {
                if left_form != right_form || left_readonly != right_readonly {
                    Decision::No
                } else {
                    self.decide_tuple_elements_equal(left_elements, right_elements)?
                }
            }
            (TypeTerm::Shape { members: left }, TypeTerm::Shape { members: right }) => {
                self.decide_shape_members_equal(left, right)?
            }
            (TypeTerm::Function(left), TypeTerm::Function(right)) => {
                self.decide_function_equal(self.terms.get(*left), self.terms.get(*right))?
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
        &self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_type_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (TypeTerm::Literal(TypeLiteralTerm::Void), target) if target.is_unit() => Decision::Yes,
            (source, TypeTerm::Literal(TypeLiteralTerm::Void)) if source.is_unit() => Decision::Yes,
            (TypeTerm::Variable(source), target) => {
                let Some(source) = self.solved_type_term(*source)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_assignable(&source, target)?
            }
            (source, TypeTerm::Variable(target)) => {
                let Some(target) = self.solved_type_term(*target)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_type_assignable(source, &target)?
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
                let form = self.decide_form_assignable(
                    self.terms.get(*source_form),
                    self.terms.get(*target_form),
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
            (
                TypeTerm::Slice {
                    element: source,
                    is_readonly: source_readonly,
                },
                TypeTerm::Slice {
                    element: target,
                    is_readonly: target_readonly,
                },
            ) => {
                if *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
                }
            }
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: source_length,
                    is_readonly: source_readonly,
                },
                TypeTerm::FixedArray {
                    element: target_element,
                    length: target_length,
                    is_readonly: target_readonly,
                },
            ) => {
                if *source_readonly && !*target_readonly {
                    Decision::No
                } else {
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
            }
            (
                TypeTerm::FixedArray {
                    element: source,
                    is_readonly: source_readonly,
                    ..
                },
                TypeTerm::Slice {
                    element: target,
                    is_readonly: target_readonly,
                },
            ) => {
                if *source_readonly && !*target_readonly {
                    Decision::No
                } else {
                    self.decide_type_relation(TypeRelation::Assignable, *source, *target)?
                }
            }
            (
                TypeTerm::Tuple {
                    form: source_form,
                    elements: source_elements,
                    is_readonly: source_readonly,
                },
                TypeTerm::Tuple {
                    form: target_form,
                    elements: target_elements,
                    is_readonly: target_readonly,
                },
            ) => {
                if source_form != target_form || (*source_readonly && !*target_readonly) {
                    Decision::No
                } else {
                    self.decide_tuple_elements_assignable(source_elements, target_elements)?
                }
            }
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: target }) => {
                self.decide_shape_assignable(source, target)?
            }
            _ => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide whether source satisfies one target constraint.
    fn decide_type_satisfies(
        &self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_type_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: target }) => {
                self.decide_shape_satisfies(source, target)?
            }
            _ => self.decide_type_assignable(source, target)?,
        };

        Ok(decision)
    }

    /// Reduce one named type reference when it names a non-nominal type alias.
    fn reduce_reference_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        reference_origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Reduction<TypeTerm>> {
        if let Some(term) = self.language_item_reference_term(symbol, arguments) {
            return Ok(Reduction::value(term));
        }

        let Some(value) = self.type_alias_body(symbol)? else {
            if let Origin::Node(source) = reference_origin
                && source.local_id.ty == dir::NodeType::Expression
                && !arguments.is_empty()
            {
                return self.reduce_function_reference_term(module, source, symbol, arguments);
            }

            return Ok(Reduction::value(TypeTerm::Reference {
                origin: reference_origin,
                symbol,
                arguments: arguments.to_vec().into(),
            }));
        };
        let value = self.intern_local_node_type_variable(symbol.module_id, value);
        let Some(term) = self.solved_type_term(value)? else {
            return Ok(Reduction::pending());
        };
        let term = if arguments.is_empty() {
            term
        } else {
            let substitution = self.generic_substitution(module, symbol, arguments)?;
            let Some(term) = term.substitute(module, &substitution, self)? else {
                return Ok(Reduction::pending());
            };

            term
        };

        let reduction = self.reduce_type_term(origin, &term)?;
        if reduction.value.is_some() {
            return Ok(reduction);
        }
        if term.is_pending_reduction() {
            return Ok(reduction);
        }

        Ok(Reduction {
            value: Some(term),
            progress: reduction.progress,
        })
    }

    /// Return the structural term for a well-known type language item reference.
    fn language_item_reference_term(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> Option<TypeTerm> {
        let item = self.environment.language.item(symbol)?;

        match item {
            dir::LanguageItem::Array => {
                let element = arguments.first()?.type_operand()?;

                Some(TypeTerm::Array { element })
            }
            dir::LanguageItem::ReadonlyArray => {
                let element = arguments.first()?.type_operand()?;
                let array = self.terms.push(TypeTerm::Array { element });
                let form = self.terms.push(FormTerm::Readonly);

                Some(TypeTerm::Form {
                    form,
                    payload: array.into(),
                })
            }
            dir::LanguageItem::FixedArray => {
                let element = arguments.first()?.type_operand()?;
                let length = arguments.get(1)?.static_operand()?;

                Some(TypeTerm::FixedArray {
                    element,
                    length,
                    is_readonly: false,
                })
            }
            dir::LanguageItem::Slice => {
                let element = arguments.first()?.type_operand()?;

                Some(TypeTerm::Slice {
                    element,
                    is_readonly: false,
                })
            }
            _ => None,
        }
    }

    /// Reduce one static value used as a type.
    fn reduce_static_value_term(
        &mut self,
        module: ModuleId,
        value: StaticOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let (value_module, term) = match value {
            StaticOperand::Variable(variable) => {
                let Some(term) = self.solved_static_term(variable)? else {
                    return Ok(None);
                };

                (variable.module, term)
            }
            StaticOperand::Term(term) => {
                let term = self.terms.get(term).clone();

                (module, term)
            }
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
            StaticTerm::Variable(variable) => {
                let Some(term) = self.solved_static_term(*variable)? else {
                    return Ok(None);
                };

                return self.type_from_static_term(module, variable.module, &term);
            }
            StaticTerm::Expression(expression) => {
                let Some(term) = self.build_static_expression_value(expression.clone())? else {
                    return Ok(None);
                };

                return self.type_from_static_term(
                    module,
                    expression.module_id,
                    &StaticTerm::Literal(term),
                );
            }
            StaticTerm::Literal(dir::StaticTerm::ScalarLiteral { value }) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            StaticTerm::Literal(dir::StaticTerm::Type { ty }) => {
                let variable = self.materialize_type_id(ty.into_global(value_module));

                TypeTerm::Variable(variable)
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
                    let ty = self.terms.push(term);

                    let member = ShapeMember::Field {
                        key: *key,
                        ty: ty.into(),
                        is_optional: false,
                        is_readonly: false,
                    };

                    members.push(member);
                }

                TypeTerm::Shape {
                    members: members.into(),
                }
            }
            StaticTerm::Literal(_) => return Ok(None),
            StaticTerm::Member { .. }
            | StaticTerm::Join { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Return the body expression for one reducible type alias.
    fn type_alias_body(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        if !self.modules.contains_key(&symbol.module_id) {
            return Ok(None);
        }
        let Some(node) = self.local_symbol_source_node(symbol) else {
            return Ok(None);
        };
        if node.ty != dir::NodeType::Declaration {
            return Ok(None);
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
        let declaration = self.module(symbol.module_id).view().get(declaration);
        let value = match declaration {
            dir::Declaration::Type(declaration) if !declaration.is_nominal => {
                Some(declaration.value)
            }
            _ => None,
        };

        Ok(value)
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
        match float {
            dir::FloatType::Float | dir::FloatType::Float64 => (value as f64) as i64 == value,
            dir::FloatType::Float32 => (value as f32) as i64 == value,
        }
    }

    /// Return whether a float literal fits one float primitive exactly.
    fn float_literal_fits_float(value: f64, float: dir::FloatType) -> bool {
        if !value.is_finite() {
            return false;
        }

        match float {
            dir::FloatType::Float | dir::FloatType::Float64 => true,
            dir::FloatType::Float32 => f64::from(value as f32) == value,
        }
    }

    /// Decide whether all source union elements assign to a target.
    fn decide_all_sources_assignable(
        &self,
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
        &self,
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
