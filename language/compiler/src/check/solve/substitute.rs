use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, AwaitTerm, CallTerm, CheckComponentState, ConstructTerm, FormTerm, FunctionTerm,
    IdentityTerm, IndexTerm, InstanceCheckTerm, KeyMembershipTerm, MappedParameterTerm,
    MemberCallTerm, MemberProtocol, OperatorTerm, ShapeMemberTerm, StaticTerm, TaggedTemplateTerm,
    TemplateTerm, TryTerm, TupleElementTerm, TypeOperationTerm, TypeTerm, VariableId,
};

/// One generic argument substitution entry.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GenericSubstitutionEntry {
    /// The generic variable being substituted.
    pub(super) variable: VariableId,
    /// The source symbol for explicit generic slots.
    pub(super) symbol: Option<dir::GlobalSymbolId>,
    /// The applied argument.
    pub(super) argument: ArgumentTerm,
}

/// Generic argument substitution for one applied owner.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GenericSubstitution {
    /// The substitution entries in declaration order.
    pub(super) entries: Vec<GenericSubstitutionEntry>,
}

impl GenericSubstitution {
    /// Return an empty substitution.
    pub(super) fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Return whether this substitution has no entries.
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the type argument for one generic variable.
    pub(super) fn type_variable(&self, variable: VariableId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.variable == variable
                && let ArgumentTerm::Type(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one generic variable.
    pub(super) fn static_variable(&self, variable: VariableId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.variable == variable
                && let ArgumentTerm::Static(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }

    /// Return the type argument for one explicit generic symbol.
    pub(super) fn type_symbol(&self, symbol: dir::GlobalSymbolId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.symbol == Some(symbol)
                && let ArgumentTerm::Type(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one explicit generic symbol.
    pub(super) fn static_symbol(&self, symbol: dir::GlobalSymbolId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.symbol == Some(symbol)
                && let ArgumentTerm::Static(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }
}

impl CheckComponentState<'_> {
    /// Substitute generic arguments through one type term.
    pub(super) fn substitute_type_term(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        term: &TypeTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match term {
            TypeTerm::Variable(variable) => {
                if let Some(argument) = substitution.type_variable(*variable) {
                    TypeTerm::Variable(argument)
                } else if let Some(term) = self.solved_type_term(*variable)? {
                    if let Some(term) = self.substitute_type_term(module, substitution, &term)? {
                        term
                    } else {
                        TypeTerm::Variable(*variable)
                    }
                } else {
                    TypeTerm::Variable(*variable)
                }
            }
            TypeTerm::Literal(dir::Type::Parameter(parameter)) => {
                if let Some(argument) = substitution.type_symbol(parameter.symbol) {
                    TypeTerm::Variable(argument)
                } else {
                    term.clone()
                }
            }
            TypeTerm::Form { form, payload } => TypeTerm::Form {
                form: self.substitute_form_term(module, substitution, form)?,
                payload: self.substitute_type_variable(module, substitution, *payload)?,
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
                        arguments: self.substitute_arguments(module, substitution, arguments)?,
                    }
                }
            }
            TypeTerm::Array { element } => TypeTerm::Array {
                element: self.substitute_type_variable(module, substitution, *element)?,
            },
            TypeTerm::Member {
                source,
                owner,
                key,
                arguments,
            } => TypeTerm::Member {
                source: *source,
                owner: self.substitute_type_variable(module, substitution, *owner)?,
                key: *key,
                arguments: self
                    .substitute_arguments(module, substitution, arguments)?
                    .into(),
            },
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => TypeTerm::FixedArray {
                element: self.substitute_type_variable(module, substitution, *element)?,
                length: self.substitute_static_variable(module, substitution, *length)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Slice {
                element,
                is_readonly,
            } => TypeTerm::Slice {
                element: self.substitute_type_variable(module, substitution, *element)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Tuple {
                form,
                elements,
                is_readonly,
            } => TypeTerm::Tuple {
                form: *form,
                elements: self.substitute_tuple_elements(module, substitution, elements)?,
                is_readonly: *is_readonly,
            },
            TypeTerm::Shape { members } => TypeTerm::Shape {
                members: self.substitute_shape_members(module, substitution, members)?,
            },
            TypeTerm::Function(function) => {
                TypeTerm::Function(self.substitute_function_term(module, substitution, function)?)
            }
            TypeTerm::Union { elements } => TypeTerm::Union {
                elements: self.substitute_type_variables(module, substitution, elements)?,
            },
            TypeTerm::Intersection { elements } => TypeTerm::Intersection {
                elements: self.substitute_type_variables(module, substitution, elements)?,
            },
            TypeTerm::Operation(operation) => TypeTerm::Operation(self.substitute_type_operation(
                module,
                substitution,
                operation,
            )?),
            TypeTerm::Call(call) => TypeTerm::Call(CallTerm {
                source: call.source,
                callee: self.substitute_type_variable(module, substitution, call.callee)?,
                member: call
                    .member
                    .as_ref()
                    .map(|member| -> CompilerResult<MemberCallTerm> {
                        Ok(MemberCallTerm {
                            receiver: self.substitute_type_variable(
                                module,
                                substitution,
                                member.receiver,
                            )?,
                            key: member.key,
                            arguments: self.substitute_arguments(
                                module,
                                substitution,
                                &member.arguments,
                            )?,
                            protocol: member
                                .protocol
                                .as_ref()
                                .map(|protocol| -> CompilerResult<MemberProtocol> {
                                    Ok(MemberProtocol {
                                        item: protocol.item,
                                        arguments: self.substitute_arguments(
                                            module,
                                            substitution,
                                            &protocol.arguments,
                                        )?,
                                    })
                                })
                                .transpose()?,
                        })
                    })
                    .transpose()?,
                candidates: call.candidates.clone(),
                generic_arguments: self.substitute_arguments(
                    module,
                    substitution,
                    &call.generic_arguments,
                )?,
                arguments: self.substitute_type_variables(module, substitution, &call.arguments)?,
            }),
            TypeTerm::Construct(construct) => TypeTerm::Construct(ConstructTerm {
                source: construct.source,
                callee: self.substitute_type_variable(module, substitution, construct.callee)?,
                generic_arguments: self.substitute_arguments(
                    module,
                    substitution,
                    &construct.generic_arguments,
                )?,
                arguments: self.substitute_type_variables(
                    module,
                    substitution,
                    &construct.arguments,
                )?,
            }),
            TypeTerm::Operator(operator) => TypeTerm::Operator(OperatorTerm {
                source: operator.source,
                kind: operator.kind,
                receiver: self.substitute_type_variable(module, substitution, operator.receiver)?,
                argument: operator
                    .argument
                    .map(|argument| self.substitute_type_variable(module, substitution, argument))
                    .transpose()?,
            }),
            TypeTerm::Index(index) => TypeTerm::Index(IndexTerm {
                source: index.source,
                receiver: self.substitute_type_variable(module, substitution, index.receiver)?,
                index: self.substitute_type_variable(module, substitution, index.index)?,
                key: index.key,
            }),
            TypeTerm::KeyMembership(membership) => TypeTerm::KeyMembership(KeyMembershipTerm {
                source: membership.source,
                key: self.substitute_type_variable(module, substitution, membership.key)?,
                receiver: self.substitute_type_variable(
                    module,
                    substitution,
                    membership.receiver,
                )?,
                static_key: membership.static_key,
            }),
            TypeTerm::InstanceCheck(instance) => TypeTerm::InstanceCheck(InstanceCheckTerm {
                source: instance.source,
                value: self.substitute_type_variable(module, substitution, instance.value)?,
                target: self.substitute_type_variable(module, substitution, instance.target)?,
            }),
            TypeTerm::Identity(identity) => TypeTerm::Identity(IdentityTerm {
                source: identity.source,
                operator: identity.operator,
                left: self.substitute_type_variable(module, substitution, identity.left)?,
                right: self.substitute_type_variable(module, substitution, identity.right)?,
            }),
            TypeTerm::Await(awaited) => TypeTerm::Await(AwaitTerm {
                source: awaited.source,
                value: self.substitute_type_variable(module, substitution, awaited.value)?,
            }),
            TypeTerm::Try(tried) => TypeTerm::Try(TryTerm {
                source: tried.source,
                value: self.substitute_type_variable(module, substitution, tried.value)?,
                kind: tried.kind,
            }),
            TypeTerm::Template(template) => TypeTerm::Template(TemplateTerm {
                source: template.source,
                strings: template.strings.clone(),
                spans: self.substitute_type_variables(module, substitution, &template.spans)?,
            }),
            TypeTerm::TaggedTemplate(template) => TypeTerm::TaggedTemplate(TaggedTemplateTerm {
                source: template.source,
                tag: self.substitute_type_variable(module, substitution, template.tag)?,
                generic_arguments: self.substitute_arguments(
                    module,
                    substitution,
                    &template.generic_arguments,
                )?,
                strings: template.strings.clone(),
                spans: self.substitute_type_variables(module, substitution, &template.spans)?,
            }),
            TypeTerm::Predicate {
                asserts,
                subject,
                target,
            } => TypeTerm::Predicate {
                asserts: *asserts,
                subject: *subject,
                target: target
                    .map(|target| self.substitute_type_variable(module, substitution, target))
                    .transpose()?,
            },
            TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::ConstAssertion
            | TypeTerm::Range { .. } => term.clone(),
        };

        Ok(Some(term))
    }

    /// Substitute generic arguments through one function term.
    pub(super) fn substitute_function_term(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        function: &FunctionTerm,
    ) -> CompilerResult<FunctionTerm> {
        Ok(FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters: function.generic_parameters.clone(),
            this_parameter: function
                .this_parameter
                .map(|parameter| self.substitute_type_variable(module, substitution, parameter))
                .transpose()?,
            parameters: self.substitute_type_variables(
                module,
                substitution,
                &function.parameters,
            )?,
            return_type: function
                .return_type
                .map(|return_type| self.substitute_type_variable(module, substitution, return_type))
                .transpose()?,
            is_generator: function.is_generator,
        })
    }

    /// Substitute generic arguments through argument terms.
    pub(super) fn substitute_arguments(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<Vec<ArgumentTerm>> {
        arguments
            .iter()
            .map(|argument| match argument {
                ArgumentTerm::Type(variable) => Ok(ArgumentTerm::Type(
                    self.substitute_type_variable(module, substitution, *variable)?,
                )),
                ArgumentTerm::Static(variable) => Ok(ArgumentTerm::Static(
                    self.substitute_static_variable(module, substitution, *variable)?,
                )),
                ArgumentTerm::SpreadType(variable) => Ok(ArgumentTerm::SpreadType(
                    self.substitute_type_variable(module, substitution, *variable)?,
                )),
                ArgumentTerm::SpreadStatic(variable) => Ok(ArgumentTerm::SpreadStatic(
                    self.substitute_static_variable(module, substitution, *variable)?,
                )),
            })
            .collect()
    }

    /// Substitute one type variable into a variable.
    fn substitute_type_variable(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        if let Some(argument) = substitution.type_variable(variable) {
            return Ok(argument);
        }
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(variable);
        };
        let Some(substituted) = self.substitute_type_term(module, substitution, &term)? else {
            return Ok(variable);
        };
        if let TypeTerm::Variable(variable) = substituted {
            return Ok(variable);
        }
        if substituted == term {
            return Ok(variable);
        }

        self.push_solved_type_variable(module, substituted)
    }

    /// Substitute type variables into variables.
    fn substitute_type_variables(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variables: &[VariableId],
    ) -> CompilerResult<Vec<VariableId>> {
        variables
            .iter()
            .map(|variable| self.substitute_type_variable(module, substitution, *variable))
            .collect()
    }

    /// Substitute one static variable into a variable.
    fn substitute_static_variable(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        if let Some(argument) = substitution.static_variable(variable) {
            return Ok(argument);
        }
        let Some(term) = self.solved_static_term(variable)? else {
            return Ok(variable);
        };
        let substituted = self.substitute_static_term(module, substitution, &term)?;
        if substituted == term {
            return Ok(variable);
        }

        self.push_solved_static_term_variable(module, substituted)
    }

    /// Substitute static variables into variables.
    fn substitute_static_variables(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variables: &[VariableId],
    ) -> CompilerResult<Vec<VariableId>> {
        variables
            .iter()
            .map(|variable| self.substitute_static_variable(module, substitution, *variable))
            .collect()
    }

    /// Substitute generic arguments through one static term.
    pub(super) fn substitute_static_term(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        term: &StaticTerm,
    ) -> CompilerResult<StaticTerm> {
        let term = match term {
            StaticTerm::Literal(dir::StaticTerm::Symbol { symbol }) => {
                if let Some(argument) = substitution.static_symbol(*symbol) {
                    if let Some(term) = self.solved_static_term(argument)? {
                        term
                    } else {
                        term.clone()
                    }
                } else {
                    term.clone()
                }
            }
            StaticTerm::Literal(dir::StaticTerm::Lifetime {
                lifetime: dir::Lifetime::Symbol(symbol),
            }) => {
                if let Some(argument) = substitution.static_symbol(*symbol) {
                    if let Some(term) = self.solved_static_term(argument)? {
                        term
                    } else {
                        term.clone()
                    }
                } else {
                    term.clone()
                }
            }
            StaticTerm::LifetimeJoin { elements } => StaticTerm::LifetimeJoin {
                elements: self.substitute_static_variables(module, substitution, elements)?,
            },
            StaticTerm::Member {
                source,
                owner,
                key,
                arguments,
            } => StaticTerm::Member {
                source: *source,
                owner: self.substitute_type_variable(module, substitution, *owner)?,
                key: *key,
                arguments: self
                    .substitute_arguments(module, substitution, arguments)?
                    .into(),
            },
            StaticTerm::Intrinsic { item, arguments } => StaticTerm::Intrinsic {
                item: *item,
                arguments: self
                    .substitute_arguments(module, substitution, arguments)?
                    .into(),
            },
            StaticTerm::Variable(_) | StaticTerm::Expression(_) | StaticTerm::Literal(_) => {
                term.clone()
            }
        };

        Ok(term)
    }

    /// Substitute generic arguments through one memory form term.
    fn substitute_form_term(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        form: &FormTerm,
    ) -> CompilerResult<FormTerm> {
        let form = match form {
            FormTerm::Borrowed { lifetime, access } => FormTerm::Borrowed {
                lifetime: self.substitute_static_variable(module, substitution, *lifetime)?,
                access: self.substitute_static_variable(module, substitution, *access)?,
            },
            FormTerm::Placed { place } => FormTerm::Placed {
                place: self.substitute_static_variable(module, substitution, *place)?,
            },
            FormTerm::Managed | FormTerm::Owned | FormTerm::Raw | FormTerm::Readonly => {
                form.clone()
            }
        };

        Ok(form)
    }

    /// Substitute generic arguments through tuple elements.
    fn substitute_tuple_elements(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        elements: &[TupleElementTerm],
    ) -> CompilerResult<Vec<TupleElementTerm>> {
        elements
            .iter()
            .map(|element| {
                Ok(TupleElementTerm {
                    label: element.label,
                    ty: self.substitute_type_variable(module, substitution, element.ty)?,
                    is_optional: element.is_optional,
                    is_readonly: element.is_readonly,
                    is_rest: element.is_rest,
                })
            })
            .collect()
    }

    /// Substitute generic arguments through shape members.
    fn substitute_shape_members(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        members: &[ShapeMemberTerm],
    ) -> CompilerResult<Vec<ShapeMemberTerm>> {
        members
            .iter()
            .map(|member| match member {
                ShapeMemberTerm::Field {
                    key,
                    ty,
                    is_optional,
                    is_readonly,
                } => Ok(ShapeMemberTerm::Field {
                    key: *key,
                    ty: self.substitute_type_variable(module, substitution, *ty)?,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }),
                ShapeMemberTerm::CallSignature { ty } => Ok(ShapeMemberTerm::CallSignature {
                    ty: self.substitute_type_variable(module, substitution, *ty)?,
                }),
                ShapeMemberTerm::ConstructSignature { ty } => {
                    Ok(ShapeMemberTerm::ConstructSignature {
                        ty: self.substitute_type_variable(module, substitution, *ty)?,
                    })
                }
                ShapeMemberTerm::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => Ok(ShapeMemberTerm::IndexSignature {
                    name: *name,
                    key_type: self.substitute_type_variable(module, substitution, *key_type)?,
                    value_type: self.substitute_type_variable(module, substitution, *value_type)?,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }),
            })
            .collect()
    }

    /// Substitute generic arguments through one type operation.
    fn substitute_type_operation(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        operation: &TypeOperationTerm,
    ) -> CompilerResult<TypeOperationTerm> {
        let operation = match operation {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => TypeOperationTerm::Conditional {
                left: self.substitute_type_variable(module, substitution, *left)?,
                right: self.substitute_type_variable(module, substitution, *right)?,
                then_type: self.substitute_type_variable(module, substitution, *then_type)?,
                else_type: self.substitute_type_variable(module, substitution, *else_type)?,
            },
            TypeOperationTerm::Index { left, index } => TypeOperationTerm::Index {
                left: self.substitute_type_variable(module, substitution, *left)?,
                index: self.substitute_type_variable(module, substitution, *index)?,
            },
            TypeOperationTerm::TemplateLiteral { strings, spans } => {
                TypeOperationTerm::TemplateLiteral {
                    strings: strings.clone(),
                    spans: self.substitute_type_variables(module, substitution, spans)?,
                }
            }
            TypeOperationTerm::Infer { name, constraint } => TypeOperationTerm::Infer {
                name: *name,
                constraint: constraint
                    .map(|constraint| {
                        self.substitute_type_variable(module, substitution, constraint)
                    })
                    .transpose()?,
            },
            TypeOperationTerm::KeyOf { target } => TypeOperationTerm::KeyOf {
                target: self.substitute_type_variable(module, substitution, *target)?,
            },
            TypeOperationTerm::Mapped {
                parameter,
                modifiers,
                value,
            } => TypeOperationTerm::Mapped {
                parameter: MappedParameterTerm {
                    name: parameter.name,
                    symbol: parameter.symbol,
                    constraint: self.substitute_type_variable(
                        module,
                        substitution,
                        parameter.constraint,
                    )?,
                    key_remap: parameter
                        .key_remap
                        .map(|key_remap| {
                            self.substitute_type_variable(module, substitution, key_remap)
                        })
                        .transpose()?,
                },
                modifiers: *modifiers,
                value: self.substitute_type_variable(module, substitution, *value)?,
            },
            TypeOperationTerm::BestCommon { elements } => TypeOperationTerm::BestCommon {
                elements: elements
                    .iter()
                    .map(|element| self.substitute_type_variable(module, substitution, *element))
                    .collect::<CompilerResult<Vec<_>>>()?,
            },
            TypeOperationTerm::Widen { source } => TypeOperationTerm::Widen {
                source: self.substitute_type_variable(module, substitution, *source)?,
            },
            TypeOperationTerm::Exclude { source, target } => TypeOperationTerm::Exclude {
                source: self.substitute_type_variable(module, substitution, *source)?,
                target: self.substitute_type_variable(module, substitution, *target)?,
            },
            TypeOperationTerm::Intrinsic { item, arguments } => TypeOperationTerm::Intrinsic {
                item: *item,
                arguments: self
                    .substitute_arguments(module, substitution, arguments)?
                    .into(),
            },
        };

        Ok(operation)
    }
}
