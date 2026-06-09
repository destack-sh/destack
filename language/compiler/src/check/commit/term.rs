use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, FormTerm, FunctionParameter, FunctionTerm, GenericArgument,
    GenericInstance, GenericParameterBinding, GenericParameterId, MemberTerm, Origin, ShapeMember,
    ShapeTerm, StaticOperand, StaticTerm, TermId, TupleElement, TypeOperand, TypeOperationTerm,
    TypeTerm, VariableId,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit source type operands into the output type table.
    pub(super) fn commit_type_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::TypeSegment> {
        let node_types = self.inputs.node_types_in(module).collect::<Vec<_>>();
        let symbol_types = self.inputs.symbol_types_in(module).collect::<Vec<_>>();

        // write source node types
        for (node, operand) in node_types {
            let Some(type_id) =
                self.commit_type_operand(module, output, environment, operand, node.local_id)?
            else {
                return Err(self.unresolved_node_type_error(module, node, operand));
            };

            output.types.set_node_type(node, type_id);
        }

        // write source symbol types
        for (symbol, operand) in symbol_types {
            let source = self
                .module(symbol.module_id)
                .symbol_declaration_node(symbol.local_id);
            let Some(type_id) =
                self.commit_type_operand(module, output, environment, operand, source)?
            else {
                return Err(self.unresolved_symbol_type_error(module, symbol, operand));
            };

            output.types.set_symbol_type(symbol, type_id);
        }

        // return the built type segment
        let state = self.module(module);
        let base = dir::TypeSegment::from_base(&state.expanded.types);
        let types = std::mem::replace(&mut output.types, base);

        Ok(types)
    }

    /// Commit source static operands into the output static table.
    pub(super) fn commit_static_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::StaticSegment> {
        let symbol_statics = self.inputs.symbol_statics_in(module).collect::<Vec<_>>();

        // write source symbol statics
        for (symbol, operand) in symbol_statics {
            let Some(static_id) =
                self.commit_closed_static_operand(module, output, environment, operand)?
            else {
                return Err(self.unresolved_symbol_static_error(module, symbol, operand));
            };

            output.statics.set_symbol_static(symbol, static_id);
        }

        // return the built static segment
        let state = self.module(module);
        let base = dir::StaticSegment::from_base(&state.expanded.statics);
        let statics = std::mem::replace(&mut output.statics, base);

        Ok(statics)
    }

    /// Commit one type operand.
    pub(in crate::check) fn commit_type_operand(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operand: TypeOperand,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let origin = Origin::Node(source.into_global(module));
        let Answer::Ready(operand) = self.normalize_type_operand(origin, operand)? else {
            return Ok(None);
        };

        match operand {
            TypeOperand::Variable(variable) => {
                self.commit_type_variable(module, output, environment, variable, source)
            }
            TypeOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                self.commit_type_term(module, output, environment, &term, source)
            }
            TypeOperand::Type(ty) => Ok(Some(ty)),
        }
    }

    /// Commit one variable as a type inside one target module.
    fn commit_type_variable(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        variable: VariableId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(term) = self.commit_variable_term(variable) else {
            return Ok(None);
        };

        self.commit_type_term(module, output, environment, &term, source)
    }

    /// Commit one type term.
    pub(super) fn commit_type_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        term: &TypeTerm,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let origin = Origin::Node(source.into_global(module));
        let can_reduce_members = source.ty == dir::NodeType::Expression;
        let Answer::Ready(reduced) =
            self.reduce_committable_type_term(origin, module, term, can_reduce_members)?
        else {
            return Ok(None);
        };
        if reduced != *term {
            return self.commit_type_term(module, output, environment, &reduced, source);
        }

        let ty = match term {
            TypeTerm::Type(ty) => return Ok(Some(*ty)),
            TypeTerm::Literal(atom) => atom.to_type(),
            TypeTerm::Intrinsic => dir::Type::Intrinsic,
            TypeTerm::Parameter(parameter) => dir::Type::Parameter(*parameter),
            TypeTerm::This => dir::Type::This,
            TypeTerm::Form { form, payload } => {
                let Some(value) =
                    self.commit_type_operand(module, output, environment, *payload, source)?
                else {
                    return Ok(None);
                };
                let Some(form) = self.commit_form_term(module, output, environment, *form)? else {
                    return Ok(None);
                };

                dir::Type::Form(dir::FormType { form, value })
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => {
                self.import_symbol_generic_template(module, *symbol)?;
                let arguments =
                    if let Some(template) = self.inference.symbol_generic_template(*symbol) {
                        self.commit_reference_arguments(
                            module,
                            output,
                            environment,
                            template,
                            arguments,
                            source,
                        )?
                    } else {
                        self.commit_argument_terms(module, output, environment, arguments, source)?
                    };
                let Some(arguments) = arguments else {
                    return Ok(None);
                };

                dir::Type::Reference(dir::ReferenceType {
                    symbol: *symbol,
                    arguments,
                })
            }
            TypeTerm::Array { element } => {
                let Some(element) =
                    self.commit_type_operand(module, output, environment, *element, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Array(dir::ArrayType { element })
            }
            TypeTerm::Member(member) => {
                let member = self.inference.term(*member).clone();
                let owner = self.member_receiver_type_operand(&member.owner);
                let Some(owner) =
                    self.commit_type_operand(module, output, environment, owner, source)?
                else {
                    return Ok(None);
                };
                let Some(arguments) = self.commit_argument_terms(
                    module,
                    output,
                    environment,
                    &member.arguments,
                    source,
                )?
                else {
                    return Ok(None);
                };

                dir::Type::Member(dir::MemberType {
                    owner,
                    key: member.key,
                    arguments,
                })
            }
            TypeTerm::StaticValue { .. } => return Ok(None),
            TypeTerm::FixedArray { element, length } => {
                let Some(element) =
                    self.commit_type_operand(module, output, environment, *element, source)?
                else {
                    return Ok(None);
                };
                let Some(count) =
                    self.commit_closed_static_operand(module, output, environment, *length)?
                else {
                    return Ok(None);
                };

                dir::Type::FixedArray(dir::FixedArrayType { element, count })
            }
            TypeTerm::Slice { element } => {
                let Some(element) =
                    self.commit_type_operand(module, output, environment, *element, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Slice(dir::SliceType { element })
            }
            TypeTerm::Tuple { form, elements } => dir::Type::Tuple(dir::TupleType {
                form: *form,
                elements: match self.commit_tuple_elements(
                    module,
                    output,
                    environment,
                    elements,
                    source,
                )? {
                    Some(elements) => elements,
                    None => return Ok(None),
                },
            }),
            TypeTerm::Shape(shape) => {
                let members = self.inference.term(*shape).members.clone();

                let Some(shape) =
                    self.commit_shape_type(module, output, environment, &members, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Shape(shape)
            }
            TypeTerm::Function(function) => {
                let Some(function) =
                    self.commit_function_type(module, output, environment, *function, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Function(function)
            }
            TypeTerm::Range {
                start,
                end,
                is_inclusive,
            } => dir::Type::Range(dir::RangeType {
                start: start.clone(),
                end: end.clone(),
                is_inclusive: *is_inclusive,
            }),
            TypeTerm::Union { elements } => {
                let Some(elements) =
                    self.commit_type_operands(module, output, environment, elements, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Union(dir::UnionType { elements })
            }
            TypeTerm::Intersection { elements } => {
                let Some(elements) =
                    self.commit_type_operands(module, output, environment, elements, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Intersection(dir::IntersectionType { elements })
            }
            TypeTerm::Operation(operation) => {
                let Some(operation) =
                    self.commit_type_operation(module, output, environment, *operation, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Operation(operation)
            }
            TypeTerm::Call(_)
            | TypeTerm::Construct(_)
            | TypeTerm::RangeValue(_)
            | TypeTerm::Tree(_)
            | TypeTerm::TypeValue(_)
            | TypeTerm::ImportMeta(_)
            | TypeTerm::Receiver(_)
            | TypeTerm::Super(_)
            | TypeTerm::Operator(_)
            | TypeTerm::Index(_)
            | TypeTerm::IndexSet(_)
            | TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Await(_)
            | TypeTerm::Try(_)
            | TypeTerm::Yield(_)
            | TypeTerm::TryFailure(_)
            | TypeTerm::Template(_)
            | TypeTerm::TaggedTemplate(_) => return Ok(None),
            TypeTerm::Dynamic { constraint } => {
                let Some(constraint) =
                    self.commit_type_operand(module, output, environment, *constraint, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Dynamic(dir::DynamicType { constraint })
            }
            TypeTerm::Closure {
                function,
                environment: capture,
            } => {
                let Some(function) =
                    self.commit_type_operand(module, output, environment, *function, source)?
                else {
                    return Ok(None);
                };
                let Some(environment) =
                    self.commit_type_operand(module, output, environment, *capture, source)?
                else {
                    return Ok(None);
                };

                dir::Type::Closure(dir::ClosureType {
                    function,
                    environment,
                })
            }
        };

        Ok(Some(
            self.intern_type(module, output, ty, source)
                .into_global(module),
        ))
    }

    /// Reduce one stable type term for commit without writing decisions.
    fn reduce_committable_type_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        can_reduce_members: bool,
    ) -> CompilerResult<Answer<TypeTerm>> {
        match term {
            TypeTerm::Member(member) if can_reduce_members => {
                let member = self.inference.term(*member).clone();

                self.reduce_committable_member_term(origin, module, &member, can_reduce_members)
            }
            TypeTerm::Shape(shape) if can_reduce_members || self.shape_has_spread(*shape) => {
                let Answer::Ready(reduced) = self.reduce_type_term(origin, term)? else {
                    return Ok(Answer::Pending);
                };

                let Some(term) = self.type_operand_term(reduced)? else {
                    return Ok(Answer::Pending);
                };

                Ok(Answer::Ready(term))
            }
            _ => Ok(Answer::Ready(term.clone())),
        }
    }

    /// Reduce one member projection for commit without selecting it.
    fn reduce_committable_member_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        member: &MemberTerm,
        can_reduce_members: bool,
    ) -> CompilerResult<Answer<TypeTerm>> {
        let owner = self.member_receiver_type_operand(&member.owner);
        let Some(owner) = self.type_operand_term(owner)? else {
            return Ok(Answer::Pending);
        };
        let Answer::Ready(owner) =
            self.reduce_committable_type_term(origin, module, &owner, can_reduce_members)?
        else {
            return Ok(Answer::Pending);
        };

        let Some(term) =
            self.resolve_member_type(origin, module, &owner, &member.key, &member.arguments)?
        else {
            return Ok(Answer::Pending);
        };
        let Some(term) = self.type_operand_term(term)? else {
            return Ok(Answer::Pending);
        };

        Ok(Answer::Ready(term))
    }

    /// Return whether one shape still contains a spread member.
    fn shape_has_spread(&self, shape: TermId<ShapeTerm>) -> bool {
        self.inference
            .term(shape)
            .members
            .iter()
            .any(|member| matches!(member, ShapeMember::Spread { .. }))
    }

    /// Commit one static term when it is closed.
    fn commit_closed_static_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        term: &StaticTerm,
    ) -> CompilerResult<Option<dir::GlobalStaticId>> {
        let term = match term {
            StaticTerm::Static(static_id) => return Ok(Some(*static_id)),
            StaticTerm::Literal(term) => term.clone(),
            StaticTerm::Parameter(parameter) => dir::StaticTerm::Parameter(*parameter),
            StaticTerm::Union { elements } => {
                let mut arguments = Vec::with_capacity(elements.len());

                // commit every static union element
                for element in elements {
                    let Some(element) =
                        self.commit_closed_static_operand(module, output, environment, *element)?
                    else {
                        return Ok(None);
                    };

                    arguments.push(element);
                }

                dir::StaticTerm::Union {
                    elements: arguments,
                }
            }
            StaticTerm::Expression(_) => return Ok(None),
            StaticTerm::Member { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => {
                return Ok(None);
            }
        };

        Ok(Some(
            self.intern_static(module, output, term).into_global(module),
        ))
    }

    /// Commit one static operand when it is closed.
    pub(in crate::check) fn commit_closed_static_operand(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operand: StaticOperand,
    ) -> CompilerResult<Option<dir::GlobalStaticId>> {
        match operand {
            StaticOperand::Variable(variable) => {
                self.commit_closed_static_variable(module, output, environment, variable)
            }
            StaticOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                self.commit_closed_static_term(module, output, environment, &term)
            }
            StaticOperand::Static(static_id) => Ok(Some(static_id)),
        }
    }

    /// Commit one static variable when its solution is closed.
    fn commit_closed_static_variable(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> CompilerResult<Option<dir::GlobalStaticId>> {
        let Some(solution) = self.variable_static_solution_operand(variable) else {
            return Ok(None);
        };

        match solution {
            StaticOperand::Variable(_) => Ok(None),
            StaticOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                self.commit_closed_static_term(module, output, environment, &term)
            }
            StaticOperand::Static(static_id) => Ok(Some(static_id)),
        }
    }

    /// Commit one memory form term.
    fn commit_form_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        form: TermId<FormTerm>,
    ) -> CompilerResult<Option<dir::Form>> {
        let form = self.inference.term(form).clone();
        let form = match form {
            FormTerm::Managed => dir::Form::Managed,
            FormTerm::Owned => dir::Form::Owned,
            FormTerm::Borrowed { lifetime, access } => {
                let Some(lifetime) =
                    self.commit_closed_static_operand(module, output, environment, lifetime)?
                else {
                    return Ok(None);
                };
                let Some(access) =
                    self.commit_closed_static_operand(module, output, environment, access)?
                else {
                    return Ok(None);
                };

                dir::Form::Borrowed { lifetime, access }
            }
            FormTerm::Raw => dir::Form::Raw,
            FormTerm::Placed { place } => {
                let Some(place) =
                    self.commit_closed_static_operand(module, output, environment, place)?
                else {
                    return Ok(None);
                };

                dir::Form::Placed { place }
            }
            FormTerm::Readonly => dir::Form::Readonly,
        };

        Ok(Some(form))
    }

    /// Return one solved type term for commit.
    fn commit_variable_term(&mut self, variable: VariableId) -> Option<TypeTerm> {
        match self.variable_type_solution_operand(variable)? {
            TypeOperand::Variable(_) => None,
            TypeOperand::Term(term) => Some(self.inference.term(term).clone()),
            TypeOperand::Type(ty) => Some(TypeTerm::Type(ty)),
        }
    }

    /// Commit generic parameters as parameter types.
    fn commit_generic_parameter_types(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        parameters: &[GenericParameterId],
        source: dir::LocalNodeIdAny,
    ) -> Vec<dir::GlobalTypeId> {
        parameters
            .iter()
            .map(|parameter| {
                let ty = dir::Type::Parameter(*parameter);

                self.intern_type(module, output, ty, source)
                    .into_global(module)
            })
            .collect()
    }

    /// Commit type operands.
    pub(in crate::check) fn commit_type_operands(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operands: &[TypeOperand],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let mut types = Vec::with_capacity(operands.len());

        // commit every operand in order
        for operand in operands {
            let Some(ty) =
                self.commit_type_operand(module, output, environment, *operand, source)?
            else {
                return Ok(None);
            };

            types.push(ty);
        }

        Ok(Some(types))
    }

    /// Commit function parameter type variables.
    pub(in crate::check) fn commit_function_parameter_type_ids(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameter],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let mut types = Vec::with_capacity(parameters.len());

        // commit every parameter type in order
        for parameter in parameters {
            let Some(ty) =
                self.commit_type_operand(module, output, environment, parameter.ty, source)?
            else {
                return Ok(None);
            };

            types.push(ty);
        }

        Ok(Some(types))
    }

    /// Commit function parameters.
    pub(in crate::check) fn commit_function_parameters(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameter],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<dir::FunctionParameterType>>> {
        let mut committed = Vec::with_capacity(parameters.len());

        // commit every parameter in order
        for parameter in parameters {
            let Some(ty) =
                self.commit_type_operand(module, output, environment, parameter.ty, source)?
            else {
                return Ok(None);
            };
            let parameter = dir::FunctionParameterType {
                ty,
                static_parameter: parameter.static_parameter,
                is_optional: parameter.is_optional,
                is_rest: parameter.is_rest,
            };

            committed.push(parameter);
        }

        Ok(Some(committed))
    }

    /// Commit one resolved generic instance.
    pub(in crate::check) fn commit_generic_instance(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        source: dir::LocalNodeIdAny,
        instance: &GenericInstance,
    ) -> CompilerResult<Option<dir::LocalGenericInstanceId>> {
        let mut generics = dir::GenericSegment::new(module);

        // borrow output tables while mutating the generic segment
        std::mem::swap(&mut output.generics, &mut generics);

        let instance = self.commit_generic_instance_in_table(
            module,
            output,
            environment,
            &mut generics,
            source,
            instance,
        );

        std::mem::swap(&mut output.generics, &mut generics);

        instance
    }

    /// Commit one resolved generic instance into one generic table.
    pub(super) fn commit_generic_instance_in_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        generics: &mut dir::GenericSegment,
        source: dir::LocalNodeIdAny,
        instance: &GenericInstance,
    ) -> CompilerResult<Option<dir::LocalGenericInstanceId>> {
        let arguments = self.commit_reference_arguments(
            module,
            output,
            environment,
            instance.template,
            &instance.arguments,
            source,
        )?;
        let Some(arguments) = arguments else {
            return Ok(None);
        };
        let instance = dir::GenericInstance::new(instance.template, arguments);
        let instance_id = generics.intern_instance(instance);
        let source = source.into_global(module);

        // attach the instance to its source node
        generics.bind_node_instance(source, instance_id);

        Ok(Some(instance_id))
    }

    /// Commit generic argument terms.
    pub(super) fn commit_argument_terms(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        arguments: &[GenericArgument],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<dir::StaticArgument>>> {
        let mut committed = Vec::with_capacity(arguments.len());

        // commit every generic argument in order
        for argument in arguments {
            let Some(argument) =
                self.commit_argument_term(module, output, environment, argument, source)?
            else {
                return Ok(None);
            };

            committed.push(argument);
        }

        Ok(Some(committed))
    }

    /// Commit generic arguments for one known template.
    fn commit_reference_arguments(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        template: dir::GlobalGenericTemplateId,
        arguments: &[GenericArgument],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<dir::StaticArgument>>> {
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, parameter)| parameter.clone())
            .collect::<Vec<_>>();
        let mut committed = Vec::with_capacity(arguments.len());

        // commit each argument with parameter context when available
        for (index, argument) in arguments.iter().enumerate() {
            let argument = if let Some(parameter) = parameters.get(index) {
                self.commit_owner_argument_term(
                    module,
                    output,
                    environment,
                    parameter,
                    argument,
                    source,
                )?
            } else {
                self.commit_argument_term(module, output, environment, argument, source)?
            };
            let Some(argument) = argument else {
                return Ok(None);
            };

            committed.push(argument);
        }

        Ok(Some(committed))
    }

    /// Commit one generic argument for one known parameter.
    fn commit_owner_argument_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        parameter: &GenericParameterBinding,
        argument: &GenericArgument,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::StaticArgument>> {
        let argument = argument.specialize(parameter, self)?;

        self.commit_argument_term(module, output, environment, &argument, source)
    }

    /// Commit one generic argument term.
    fn commit_argument_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        argument: &GenericArgument,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::StaticArgument>> {
        let value = match argument {
            GenericArgument::Type(operand) => {
                let Some(ty) =
                    self.commit_type_operand(module, output, environment, *operand, source)?
                else {
                    return Ok(None);
                };

                self.commit_type_argument(module, output, ty).value
            }
            GenericArgument::Static(operand) => {
                let Some(value) =
                    self.commit_closed_static_operand(module, output, environment, *operand)?
                else {
                    return Ok(None);
                };

                value
            }
            GenericArgument::AssociatedType { name, value } => {
                let Some(ty) =
                    self.commit_type_operand(module, output, environment, *value, source)?
                else {
                    return Ok(None);
                };
                let value = self.commit_type_argument(module, output, ty).value;

                return Ok(Some(dir::StaticArgument {
                    name: Some(*name),
                    value,
                }));
            }
            GenericArgument::AssociatedConst { name, value } => {
                let Some(value) =
                    self.commit_closed_static_operand(module, output, environment, *value)?
                else {
                    return Ok(None);
                };

                return Ok(Some(dir::StaticArgument {
                    name: Some(*name),
                    value,
                }));
            }
            GenericArgument::TypeOrStatic { .. }
            | GenericArgument::SpreadType(_)
            | GenericArgument::SpreadStatic(_)
            | GenericArgument::SpreadTypeOrStatic { .. } => return Ok(None),
        };

        Ok(Some(dir::StaticArgument::value(value)))
    }

    /// Commit one type as a static argument.
    fn commit_type_argument(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        ty: dir::GlobalTypeId,
    ) -> dir::StaticArgument {
        let value = self.intern_static(module, output, dir::StaticTerm::Type { ty });

        dir::StaticArgument::value(value.into_global(module))
    }

    /// Commit tuple element terms.
    fn commit_tuple_elements(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        elements: &[TupleElement],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<Vec<dir::TypeElement>>> {
        let mut committed = Vec::with_capacity(elements.len());

        // commit every tuple element in order
        for element in elements {
            let Some(ty) =
                self.commit_type_operand(module, output, environment, element.ty, source)?
            else {
                return Ok(None);
            };
            let element = dir::TypeElement {
                label: element.label,
                ty,
                is_optional: element.is_optional,
                is_readonly: element.is_readonly,
                is_rest: element.is_rest,
            };

            committed.push(element);
        }

        Ok(Some(committed))
    }

    /// Commit one structural shape type.
    fn commit_shape_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        members: &[ShapeMember],
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::ShapeType>> {
        let mut fields = Vec::new();
        let mut call_signatures = Vec::new();
        let mut construct_signatures = Vec::new();
        let mut index_signatures = Vec::new();

        // collect solved shape members
        for member in members {
            let member = member.clone();
            match member {
                ShapeMember::Field {
                    key,
                    ty,
                    is_optional,
                    is_readonly,
                } => fields.push(dir::TypeField {
                    key,
                    ty: match self.commit_type_operand(module, output, environment, ty, source)? {
                        Some(ty) => ty,
                        None => return Ok(None),
                    },
                    is_optional,
                    is_readonly,
                }),
                ShapeMember::Spread { .. } => return Ok(None),
                ShapeMember::CallSignature { ty } => {
                    let Some(ty) =
                        self.commit_type_operand(module, output, environment, ty, source)?
                    else {
                        return Ok(None);
                    };

                    call_signatures.push(ty);
                }
                ShapeMember::ConstructSignature { ty } => {
                    let Some(ty) =
                        self.commit_type_operand(module, output, environment, ty, source)?
                    else {
                        return Ok(None);
                    };

                    construct_signatures.push(ty);
                }
                ShapeMember::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => {
                    let Some(key_type) =
                        self.commit_type_operand(module, output, environment, key_type, source)?
                    else {
                        return Ok(None);
                    };
                    let Some(value_type) =
                        self.commit_type_operand(module, output, environment, value_type, source)?
                    else {
                        return Ok(None);
                    };

                    index_signatures.push(dir::TypeIndexSignature {
                        name,
                        key_type,
                        value_type,
                        is_optional,
                        is_readonly,
                    });
                }
            }
        }

        Ok(Some(dir::ShapeType {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        }))
    }

    /// Commit one function type.
    fn commit_function_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        function: TermId<FunctionTerm>,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::FunctionType>> {
        let function = self.inference.term(function).clone();
        let generic_parameters = self.commit_generic_parameter_types(
            module,
            output,
            &function.generic_parameters,
            source,
        );
        let this_parameter = if let Some(parameter) = function.this_parameter {
            self.commit_type_operand(module, output, environment, parameter, source)?
        } else {
            None
        };
        let Some(parameters) = self.commit_function_parameters(
            module,
            output,
            environment,
            &function.parameters,
            source,
        )?
        else {
            return Ok(None);
        };
        let return_type = if let Some(return_type) = function.return_type {
            self.commit_type_operand(module, output, environment, return_type, source)?
        } else {
            None
        };

        Ok(Some(dir::FunctionType {
            asynchrony: function.asynchrony,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
            is_generator: function.is_generator,
        }))
    }

    /// Commit one type operation.
    fn commit_type_operation(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operation: TermId<TypeOperationTerm>,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::TypeOperation>> {
        let operation = self.inference.term(operation).clone();
        let operation = match operation {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                let Some(left) =
                    self.commit_type_operand(module, output, environment, left, source)?
                else {
                    return Ok(None);
                };
                let Some(right) =
                    self.commit_type_operand(module, output, environment, right, source)?
                else {
                    return Ok(None);
                };
                let Some(then_type) =
                    self.commit_type_operand(module, output, environment, then_type, source)?
                else {
                    return Ok(None);
                };
                let Some(else_type) =
                    self.commit_type_operand(module, output, environment, else_type, source)?
                else {
                    return Ok(None);
                };

                dir::TypeOperation::Conditional(dir::ConditionalType {
                    left,
                    right,
                    then_type,
                    else_type,
                })
            }
            TypeOperationTerm::Index { left, index } => {
                let Some(left) =
                    self.commit_type_operand(module, output, environment, left, source)?
                else {
                    return Ok(None);
                };
                let Some(index) =
                    self.commit_type_operand(module, output, environment, index, source)?
                else {
                    return Ok(None);
                };

                dir::TypeOperation::Index(dir::IndexType { left, index })
            }
            TypeOperationTerm::TemplateLiteral { strings, spans } => {
                let Some(spans) =
                    self.commit_type_operands(module, output, environment, &spans, source)?
                else {
                    return Ok(None);
                };

                dir::TypeOperation::TemplateLiteral(dir::TemplateLiteralType { strings, spans })
            }
            TypeOperationTerm::Infer { name, constraint } => {
                let constraint = if let Some(constraint) = constraint {
                    self.commit_type_operand(module, output, environment, constraint, source)?
                } else {
                    None
                };

                dir::TypeOperation::Infer(dir::InferType { name, constraint })
            }
            TypeOperationTerm::KeyOf { target } => {
                let Some(target) =
                    self.commit_type_operand(module, output, environment, target, source)?
                else {
                    return Ok(None);
                };

                dir::TypeOperation::KeyOf(dir::UnaryType { target })
            }
            TypeOperationTerm::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let name = parameter.name;
                let symbol = parameter.symbol;
                let constraint = parameter.constraint;
                let key_remap = parameter.key_remap;
                let Some(constraint) =
                    self.commit_type_operand(module, output, environment, constraint, source)?
                else {
                    return Ok(None);
                };
                let key_remap = if let Some(key_remap) = key_remap {
                    self.commit_type_operand(module, output, environment, key_remap, source)?
                } else {
                    None
                };
                let Some(value) =
                    self.commit_type_operand(module, output, environment, value, source)?
                else {
                    return Ok(None);
                };

                dir::TypeOperation::Mapped(dir::MappedType {
                    parameter: dir::MappedTypeParameter {
                        name,
                        symbol,
                        constraint,
                        key_remap,
                    },
                    modifiers,
                    value,
                })
            }
            TypeOperationTerm::StringMapping { mapping, argument } => {
                let Some(target) =
                    self.commit_type_operand(module, output, environment, argument, source)?
                else {
                    return Ok(None);
                };

                dir::TypeOperation::StringMapping { mapping, target }
            }
            TypeOperationTerm::BestCommon { .. }
            | TypeOperationTerm::Widen { .. }
            | TypeOperationTerm::Exclude { .. }
            | TypeOperationTerm::NarrowMember { .. }
            | TypeOperationTerm::Intrinsic { .. } => return Ok(None),
        };

        Ok(Some(operation))
    }
}
