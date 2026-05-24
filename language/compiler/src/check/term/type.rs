use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, AwaitTerm, CallTerm, CheckComponentState, ConstructTerm, Decision, FormTerm,
    FunctionTerm, GenericSubstitution, IdentityTerm, IndexTerm, IndexWriteTerm, InstanceCheckTerm,
    KeyMembershipTerm, MemberCallTerm, MemberProtocol, OperatorTerm, Progress, ShapeMemberTerm,
    Solution, StaticRelation, TaggedTemplateTerm, TemplateTerm, TryTerm, TupleElementTerm,
    TypeOperationTerm, TypeRelation, VariableId, VariableOrigin,
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
    Parameter {
        /// The referenced generic parameter symbol.
        symbol: dir::GlobalSymbolId,
    },
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
    /// Runtime index write.
    ///
    /// ```ts
    /// value[key] = next
    /// ```
    IndexWrite(IndexWriteTerm),
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
            Self::IndexWrite(write) => variables.extend(write.referenced_variables()),
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
            Self::Dynamic { constraint } => variables.push(*constraint),
            Self::Closure {
                function,
                environment,
            } => {
                variables.push(*function);
                variables.push(*environment);
            }
            Self::Literal(_)
            | Self::Parameter { symbol: _ }
            | Self::This
            | Self::Intrinsic
            | Self::ConstAssertion => {}
        }

        variables
    }
}

impl CheckComponentState<'_> {
    /// Apply expected literal type context to one expression variable.
    pub(in crate::check) fn expect_literal_type(
        &mut self,
        variable: VariableId,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(_)) = source else {
            return Ok(Progress::Unchanged);
        };
        let TypeTerm::Literal(target_type) = target else {
            return Ok(Progress::Unchanged);
        };
        if !Self::can_expect_literal(target_type) {
            return Ok(Progress::Unchanged);
        }
        if self.decide_type_term_relation(TypeRelation::Assignable, source, target)?
            != Decision::Yes
        {
            return Ok(Progress::Unchanged);
        }
        let check_module = self.module_mut(variable.module)?;
        let check_variable = check_module.variable_mut(variable);
        if !matches!(check_variable.origin, VariableOrigin::Node(_)) {
            return Ok(Progress::Unchanged);
        }

        check_variable.solution = Some(Solution::Type(target.clone()));

        Ok(Progress::changed(variable))
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
            TypeRelation::Satisfies | TypeRelation::Extends | TypeRelation::Implements => {
                self.decide_type_assignable(left, right)?
            }
        };

        Ok(decision)
    }

    /// Decide one solved type relation.
    pub(in crate::check) fn decide_type_relation(
        &self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(left) = self.solved_type_term(left)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(right) = self.solved_type_term(right)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(relation, &left, &right)
    }

    /// Apply an expected result type to the term that defines it.
    pub(in crate::check) fn expect_type_term(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let upper_bounds = self
            .module(result.module)?
            .variable(result)
            .upper_bounds
            .clone();
        let mut progress = Progress::Unchanged;

        // push the exact result when it is known
        let Some(result_term) = self.solved_type_term(result)? else {
            return self.expect_type_term_upper_bounds(result, term, &upper_bounds);
        };
        progress = progress.merge(self.expect_type_term_result(
            result,
            result,
            &result_term,
            term,
            TypeExpectationMode::Exact,
        )?);

        // push solved upper bounds as contextual expectations
        progress =
            progress.merge(self.expect_type_term_upper_bounds(result, term, &upper_bounds)?);

        Ok(progress)
    }

    /// Reduce one type term when the solver has enough input.
    pub(in crate::check) fn reduce_type_term(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match term {
            TypeTerm::Variable(variable) => self.solved_type_term(*variable)?,
            TypeTerm::Member {
                source,
                owner,
                key,
                arguments,
            } if arguments.is_empty() => self.reduce_member_type(module, *source, *owner, *key)?,
            TypeTerm::Operation(TypeOperationTerm::Exclude { source, target }) => {
                self.reduce_exclude_type(module, *source, *target)?
            }
            TypeTerm::Operation(TypeOperationTerm::BestCommon { elements }) => {
                self.reduce_best_common_type(module, elements)?
            }
            TypeTerm::Operation(TypeOperationTerm::Widen { source }) => {
                self.reduce_widen_type(*source)?
            }
            TypeTerm::Operation(TypeOperationTerm::Intrinsic { item, arguments }) => {
                self.reduce_memory_type(module, *item, arguments)?
            }
            TypeTerm::Reference {
                source,
                symbol,
                arguments,
            } => self.reduce_named_type(module, *source, *symbol, arguments)?,
            TypeTerm::Call(call) => self.reduce_call_type(module, call)?,
            TypeTerm::Construct(construct) => self.reduce_construct_type(module, construct)?,
            TypeTerm::Operator(operator) => self.reduce_operator_type(operator)?,
            TypeTerm::Index(index) => self.reduce_index_type(module, index)?,
            TypeTerm::IndexWrite(write) => self.reduce_index_write_type(module, write)?,
            TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_)
            | TypeTerm::TaggedTemplate(_) => None,
            TypeTerm::Await(awaited) => self.reduce_await_type(awaited)?,
            TypeTerm::Try(tried) => self.reduce_try_type(module, tried)?,
            TypeTerm::Member { .. } => None,
            term => Some(term.clone()),
        };

        Ok(term)
    }

    /// Decompose equality between solved type terms into smaller relations.
    pub(in crate::check) fn relate_solved_type_equal(
        &mut self,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (left, right) {
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
                let form = self.relate_form_equal(left_form, right_form)?;
                let value = self.relate_type_equal(*left_value, *right_value)?;

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
            ) => self.relate_type_equal(*left_element, *right_element)?,
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
                let element = self.relate_type_equal(*left_element, *right_element)?;
                let length = self.relate_static_equal(*left_length, *right_length)?;

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
                    self.relate_tuple_elements_equal(left, right)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape { members: left }, TypeTerm::Shape { members: right }) => {
                self.relate_shape_members_equal(left, right)?
            }
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decompose assignability between solved type terms into smaller relations.
    pub(in crate::check) fn relate_solved_type_assignable(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (source, target) {
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
                let form = self.relate_form_assignable(source_form, target_form)?;
                let value = self.relate_type_assignable(*source_value, *target_value)?;

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
            ) => self.relate_type_assignable(*source_element, *target_element)?,
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
            ) => self.relate_type_assignable(*source_element, *target_element)?,
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
                let element = self.relate_type_assignable(*source_element, *target_element)?;
                let length = self.relate_static_equal(*source_length, *target_length)?;

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
                    self.relate_tuple_elements_assignable(source, target)?
                } else {
                    Progress::Unchanged
                }
            }
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: target }) => {
                self.relate_shape_members_assignable(source, target)?
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
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match self {
            TypeTerm::Variable(variable) => {
                if let Some(argument) = substitution.type_variable(*variable) {
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
            TypeTerm::Parameter { symbol } => {
                if let Some(argument) = substitution.type_symbol(*symbol) {
                    TypeTerm::Variable(argument)
                } else {
                    self.clone()
                }
            }
            TypeTerm::Form { form, payload } => TypeTerm::Form {
                form: form.substitute(module, substitution, state)?,
                payload: state.substitute_type_variable(module, substitution, *payload)?,
            },
            TypeTerm::Reference {
                source,
                symbol,
                arguments,
            } => {
                if arguments.is_empty()
                    && let Some(argument) = substitution.type_symbol(*symbol)
                {
                    TypeTerm::Variable(argument)
                } else {
                    TypeTerm::Reference {
                        source: *source,
                        symbol: *symbol,
                        arguments: ArgumentTerm::substitute_all(
                            arguments,
                            module,
                            substitution,
                            state,
                        )?,
                    }
                }
            }
            TypeTerm::Array { element } => TypeTerm::Array {
                element: state.substitute_type_variable(module, substitution, *element)?,
            },
            TypeTerm::Member {
                source,
                owner,
                key,
                arguments,
            } => TypeTerm::Member {
                source: *source,
                owner: state.substitute_type_variable(module, substitution, *owner)?,
                key: *key,
                arguments: ArgumentTerm::substitute_all(arguments, module, substitution, state)?
                    .into(),
            },
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => TypeTerm::FixedArray {
                element: state.substitute_type_variable(module, substitution, *element)?,
                length: state.substitute_static_variable(module, substitution, *length)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Slice {
                element,
                is_readonly,
            } => TypeTerm::Slice {
                element: state.substitute_type_variable(module, substitution, *element)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Tuple {
                form,
                elements,
                is_readonly,
            } => TypeTerm::Tuple {
                form: *form,
                elements: TupleElementTerm::substitute_all(elements, module, substitution, state)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Shape { members } => TypeTerm::Shape {
                members: ShapeMemberTerm::substitute_all(members, module, substitution, state)?,
            },
            TypeTerm::Function(function) => {
                TypeTerm::Function(function.substitute(module, substitution, state)?)
            }
            TypeTerm::Union { elements } => TypeTerm::Union {
                elements: state.substitute_type_variables(module, substitution, elements)?,
            },
            TypeTerm::Intersection { elements } => TypeTerm::Intersection {
                elements: state.substitute_type_variables(module, substitution, elements)?,
            },
            TypeTerm::Operation(operation) => {
                TypeTerm::Operation(operation.substitute(module, substitution, state)?)
            }
            TypeTerm::Call(call) => TypeTerm::Call(CallTerm {
                source: call.source,
                callee: state.substitute_type_variable(module, substitution, call.callee)?,
                member: call
                    .member
                    .as_ref()
                    .map(|member| -> CompilerResult<MemberCallTerm> {
                        Ok(MemberCallTerm {
                            receiver: state.substitute_type_variable(
                                module,
                                substitution,
                                member.receiver,
                            )?,
                            key: member.key,
                            arguments: ArgumentTerm::substitute_all(
                                &member.arguments,
                                module,
                                substitution,
                                state,
                            )?,
                            protocol: member
                                .protocol
                                .as_ref()
                                .map(|protocol| -> CompilerResult<MemberProtocol> {
                                    Ok(MemberProtocol {
                                        item: protocol.item,
                                        arguments: ArgumentTerm::substitute_all(
                                            &protocol.arguments,
                                            module,
                                            substitution,
                                            state,
                                        )?,
                                    })
                                })
                                .transpose()?,
                        })
                    })
                    .transpose()?,
                candidates: call.candidates.clone(),
                generic_arguments: ArgumentTerm::substitute_all(
                    &call.generic_arguments,
                    module,
                    substitution,
                    state,
                )?,
                arguments: state.substitute_type_variables(
                    module,
                    substitution,
                    &call.arguments,
                )?,
            }),
            TypeTerm::Construct(construct) => TypeTerm::Construct(ConstructTerm {
                source: construct.source,
                callee: state.substitute_type_variable(module, substitution, construct.callee)?,
                generic_arguments: ArgumentTerm::substitute_all(
                    &construct.generic_arguments,
                    module,
                    substitution,
                    state,
                )?,
                arguments: state.substitute_type_variables(
                    module,
                    substitution,
                    &construct.arguments,
                )?,
            }),
            TypeTerm::Operator(operator) => TypeTerm::Operator(OperatorTerm {
                source: operator.source,
                kind: operator.kind,
                receiver: state.substitute_type_variable(
                    module,
                    substitution,
                    operator.receiver,
                )?,
                argument: operator
                    .argument
                    .map(|argument| state.substitute_type_variable(module, substitution, argument))
                    .transpose()?,
            }),
            TypeTerm::Index(index) => TypeTerm::Index(IndexTerm {
                source: index.source,
                receiver: state.substitute_type_variable(module, substitution, index.receiver)?,
                index: state.substitute_type_variable(module, substitution, index.index)?,
                key: index.key,
            }),
            TypeTerm::IndexWrite(write) => TypeTerm::IndexWrite(IndexWriteTerm {
                source: write.source,
                receiver: state.substitute_type_variable(module, substitution, write.receiver)?,
                index: state.substitute_type_variable(module, substitution, write.index)?,
                value: state.substitute_type_variable(module, substitution, write.value)?,
                key: write.key,
            }),
            TypeTerm::KeyMembership(membership) => TypeTerm::KeyMembership(KeyMembershipTerm {
                source: membership.source,
                key: state.substitute_type_variable(module, substitution, membership.key)?,
                receiver: state.substitute_type_variable(
                    module,
                    substitution,
                    membership.receiver,
                )?,
                static_key: membership.static_key,
            }),
            TypeTerm::InstanceCheck(instance) => TypeTerm::InstanceCheck(InstanceCheckTerm {
                source: instance.source,
                value: state.substitute_type_variable(module, substitution, instance.value)?,
                target: state.substitute_type_variable(module, substitution, instance.target)?,
            }),
            TypeTerm::Identity(identity) => TypeTerm::Identity(IdentityTerm {
                source: identity.source,
                operator: identity.operator,
                left: state.substitute_type_variable(module, substitution, identity.left)?,
                right: state.substitute_type_variable(module, substitution, identity.right)?,
            }),
            TypeTerm::Await(awaited) => TypeTerm::Await(AwaitTerm {
                source: awaited.source,
                value: state.substitute_type_variable(module, substitution, awaited.value)?,
            }),
            TypeTerm::Try(tried) => TypeTerm::Try(TryTerm {
                source: tried.source,
                value: state.substitute_type_variable(module, substitution, tried.value)?,
                kind: tried.kind,
            }),
            TypeTerm::Template(template) => TypeTerm::Template(TemplateTerm {
                source: template.source,
                strings: template.strings.clone(),
                spans: state.substitute_type_variables(module, substitution, &template.spans)?,
            }),
            TypeTerm::TaggedTemplate(template) => TypeTerm::TaggedTemplate(TaggedTemplateTerm {
                source: template.source,
                tag: state.substitute_type_variable(module, substitution, template.tag)?,
                generic_arguments: ArgumentTerm::substitute_all(
                    &template.generic_arguments,
                    module,
                    substitution,
                    state,
                )?,
                strings: template.strings.clone(),
                spans: state.substitute_type_variables(module, substitution, &template.spans)?,
            }),
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

impl CheckComponentState<'_> {
    /// Decide exact equality for type variable lists.
    pub(in crate::check) fn decide_type_variable_list_equal(
        &self,
        left: &[VariableId],
        right: &[VariableId],
    ) -> CompilerResult<Decision> {
        if left.len() != right.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // compare matching variables in declaration order
        for (left, right) in left.iter().zip(right) {
            decision =
                decision.and(self.decide_type_relation(TypeRelation::Equal, *left, *right)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality for optional type variables.
    pub(in crate::check) fn decide_optional_type_variable_equal(
        &self,
        left: Option<VariableId>,
        right: Option<VariableId>,
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

    /// Apply solved upper bounds to the term that defines a variable.
    fn expect_type_term_upper_bounds(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
        upper_bounds: &[VariableId],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // apply each solved upper bound independently
        for upper_bound in upper_bounds {
            let Some(expected) = self.solved_type_term(*upper_bound)? else {
                continue;
            };
            progress = progress.merge(self.expect_type_term_result(
                result,
                *upper_bound,
                &expected,
                term,
                TypeExpectationMode::UpperBound,
            )?);
        }

        Ok(progress)
    }

    /// Apply one expected result type to the term that defines it.
    fn expect_type_term_result(
        &mut self,
        result: VariableId,
        expected: VariableId,
        expected_term: &TypeTerm,
        term: &TypeTerm,
        mode: TypeExpectationMode,
    ) -> CompilerResult<Progress> {
        let progress = match term {
            TypeTerm::Variable(variable) => match mode {
                TypeExpectationMode::Exact => self.relate_type_equal(*variable, expected)?,
                TypeExpectationMode::UpperBound => {
                    self.relate_type_assignable(*variable, expected)?
                }
            },
            TypeTerm::Form { form, payload } => {
                let TypeTerm::Form {
                    form: result_form,
                    payload: result_payload,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let form = self.expect_form(form, &result_form)?;
                let payload = self.relate_type_assignable(*payload, *result_payload)?;

                form.merge(payload)
            }
            TypeTerm::Array { element } => {
                let TypeTerm::Array {
                    element: result_element,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.relate_type_assignable(*element, *result_element)?
            }
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

                self.relate_type_assignable(*element, *result_element)?
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
                let element = self.relate_type_assignable(*element, *result_element)?;
                let length = self.relate_static_equal(*length, *result_length)?;

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

                self.expect_tuple_elements(elements, &result_elements)?
            }
            TypeTerm::Shape { members } => {
                let TypeTerm::Shape {
                    members: result_members,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_shape_members(members, &result_members)?
            }
            TypeTerm::Call(call) => self.expect_call_result(call, expected)?,
            TypeTerm::Construct(construct) => {
                self.expect_construct_result(result.module, construct, expected)?
            }
            TypeTerm::Operator(operator) => self.expect_operator_result(operator, expected)?,
            TypeTerm::Index(index) => self.expect_index_result(index, expected)?,
            TypeTerm::IndexWrite(write) => self.expect_index_write_result(write, expected)?,
            TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_)
            | TypeTerm::TaggedTemplate(_) => Progress::Unchanged,
            TypeTerm::Await(awaited) => self.expect_await_result(awaited, expected)?,
            TypeTerm::Try(tried) => self.expect_try_result(tried, expected)?,
            TypeTerm::Operation(TypeOperationTerm::Exclude { source, target: _ }) => {
                self.relate_type_assignable(*source, expected)?
            }
            TypeTerm::Operation(TypeOperationTerm::Conditional { .. })
            | TypeTerm::Operation(TypeOperationTerm::Index { .. })
            | TypeTerm::Operation(TypeOperationTerm::TemplateLiteral { .. })
            | TypeTerm::Operation(TypeOperationTerm::Infer { .. })
            | TypeTerm::Operation(TypeOperationTerm::KeyOf { .. })
            | TypeTerm::Operation(TypeOperationTerm::Mapped { .. })
            | TypeTerm::Operation(TypeOperationTerm::BestCommon { .. })
            | TypeTerm::Operation(TypeOperationTerm::Widen { .. })
            | TypeTerm::Operation(TypeOperationTerm::Intrinsic { .. })
            | TypeTerm::Member { .. }
            | TypeTerm::Reference { .. }
            | TypeTerm::Function(_)
            | TypeTerm::Range { .. }
            | TypeTerm::Union { .. }
            | TypeTerm::Intersection { .. }
            | TypeTerm::Predicate { .. }
            | TypeTerm::Dynamic { .. }
            | TypeTerm::Closure { .. }
            | TypeTerm::Parameter { .. }
            | TypeTerm::This
            | TypeTerm::Intrinsic
            | TypeTerm::ConstAssertion
            | TypeTerm::Literal(_) => Progress::Unchanged,
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
                let form = self.decide_form_equal(left_form, right_form)?;
                if form != Decision::Yes {
                    return Ok(form);
                }

                self.decide_type_relation(TypeRelation::Equal, *left_value, *right_value)?
            }
            (TypeTerm::Literal(left), TypeTerm::Literal(right)) => {
                self.decide_type_atom_equal(left, right)
            }
            (
                TypeTerm::Reference {
                    source: _,
                    symbol: left_symbol,
                    arguments: left_arguments,
                },
                TypeTerm::Reference {
                    source: _,
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
                self.decide_function_equal(left, right)?
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
                let form = self.decide_form_assignable(source_form, target_form)?;
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

    /// Reduce one named type reference when it names a non-nominal type alias.
    fn reduce_named_type(
        &mut self,
        module: ModuleId,
        source: Option<dir::GlobalNodeIdAny>,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(value) = self.type_alias_body(symbol)? else {
            return Ok(Some(TypeTerm::Reference {
                source,
                symbol,
                arguments: arguments.to_vec(),
            }));
        };
        let value = self
            .module_mut(symbol.module_id)?
            .type_expression_variable(value);
        let Some(term) = self.solved_type_term(value)? else {
            return Ok(None);
        };
        if arguments.is_empty() {
            return Ok(Some(term));
        }
        let substitution = self.generic_substitution(symbol, arguments)?;
        let term = term.substitute(module, &substitution, self)?;

        Ok(term)
    }

    /// Return the body expression for one reducible type alias.
    fn type_alias_body(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        if !self.modules.contains_key(&symbol.module_id) {
            return Ok(None);
        }
        let module = self.module(symbol.module_id)?;
        let Some(node) = module.symbol_source_node(symbol) else {
            return Ok(None);
        };
        if node.ty != dir::NodeType::Declaration {
            return Ok(None);
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
        let declaration = module.input.view().get(declaration);
        let value = match declaration {
            dir::Declaration::Type(declaration) if !declaration.is_nominal => {
                Some(declaration.value)
            }
            _ => None,
        };

        Ok(value)
    }

    /// Decide exact atom equality.
    fn decide_type_atom_equal(&self, left: &TypeLiteralTerm, right: &TypeLiteralTerm) -> Decision {
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

    /// Return whether a literal can take one expected atom.
    fn can_expect_literal(target: &TypeLiteralTerm) -> bool {
        matches!(
            target,
            TypeLiteralTerm::Null | TypeLiteralTerm::Object | TypeLiteralTerm::Primitive(_)
        )
    }

    /// Decide whether all source union elements assign to a target.
    fn decide_all_sources_assignable(
        &self,
        sources: &[VariableId],
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        for source in sources {
            let Some(source) = self.solved_type_term(*source)? else {
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
        targets: &[VariableId],
    ) -> CompilerResult<Decision> {
        let mut has_unknown = false;

        // accept the first matching target
        for target in targets {
            let Some(target) = self.solved_type_term(*target)? else {
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
