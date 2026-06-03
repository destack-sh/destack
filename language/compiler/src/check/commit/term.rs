use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Constraint, FormTerm, FunctionParameter, FunctionTerm, GenericArgument,
    GenericInstance, MemberTerm, Origin, ShapeMember, ShapeTerm, StaticOperand, StaticTerm, TermId,
    TupleElement, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, VariableId,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit source type operands into the output type table.
    pub(super) fn commit_type_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) {
        let node_types = self.inputs.node_types_in(module).collect::<Vec<_>>();
        let symbol_types = self.inputs.symbol_types_in(module).collect::<Vec<_>>();

        // write source node types
        for (node, operand) in node_types {
            let Some(type_id) =
                self.commit_type_operand(module, output, environment, operand, node.local_id)
            else {
                self.panic_unresolved_node_type(module, node, operand);
            };

            output.types.set_node_type(node, type_id);
        }

        // write source symbol types
        for (symbol, operand) in symbol_types {
            let source = self
                .module(symbol.module_id)
                .symbol_declaration_node(symbol.local_id);
            let Some(type_id) =
                self.commit_type_operand(module, output, environment, operand, source)
            else {
                self.panic_unresolved_symbol_type(module, symbol, operand);
            };

            output.types.set_symbol_type(symbol, type_id);
        }
    }

    /// Commit source static operands into the output static table.
    pub(super) fn commit_static_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) {
        let symbol_statics = self.inputs.symbol_statics_in(module).collect::<Vec<_>>();

        // write source symbol statics
        for (symbol, operand) in symbol_statics {
            let Some(static_id) = self.commit_static_operand(module, output, environment, operand)
            else {
                self.panic_unresolved_symbol_static(module, symbol, operand);
            };

            output.statics.set_symbol_static(symbol, static_id);
        }
    }

    /// Return the output type already attached to one operand.
    fn committed_operand_type(
        &self,
        output: &CheckModuleOutput,
        operand: TypeOperand,
    ) -> Option<dir::GlobalTypeId> {
        match operand {
            TypeOperand::Variable(variable) => {
                let symbol = self.variable_source_symbol(variable)?;

                output.types.get_symbol_type_id(symbol)
            }
            TypeOperand::Term(_) => None,
            TypeOperand::Type(ty) => Some(ty),
        }
    }

    /// Commit the solved type for one variable.
    pub(in crate::check) fn commit_variable_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::GlobalTypeId> {
        if let Some(type_id) = self.committed_operand_type(output, TypeOperand::Variable(variable))
        {
            return Some(type_id);
        }
        let source = self.variable_source_node(variable);

        self.commit_type_variable(module, output, environment, variable, source)
    }

    /// Commit one type operand.
    pub(in crate::check) fn commit_type_operand(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operand: TypeOperand,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        match operand {
            TypeOperand::Variable(variable) => {
                self.commit_type_variable(module, output, environment, variable, source)
            }
            TypeOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                self.commit_type_term(module, output, environment, &term, source)
            }
            TypeOperand::Type(ty) => Some(ty),
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
    ) -> Option<dir::GlobalTypeId> {
        match self.commit_variable_term(variable) {
            Some(term) if term.referenced_variables(self).is_empty() => {
                self.commit_type_term(module, output, environment, &term, source)
            }
            None => None,
            Some(_) => None,
        }
    }

    /// Commit the declared type term for one variable.
    pub(in crate::check) fn commit_declared_type_variable(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::GlobalTypeId> {
        if let Some(type_id) = self.committed_operand_type(output, TypeOperand::Variable(variable))
        {
            return Some(type_id);
        }

        let term = self.declared_type_variable_term(variable)?;
        let source = self.variable_source_node(variable);

        self.commit_type_term(module, output, environment, &term, source)
    }

    /// Return the declared type term for one variable.
    fn declared_type_variable_term(&self, variable: VariableId) -> Option<TypeTerm> {
        self.inference.constraints().find_map(|constraint| {
            let Constraint::Type {
                relation: TypeRelation::Equal,
                left,
                right,
                origin: _,
                condition: _,
            } = constraint
            else {
                return None;
            };

            match (left, right) {
                (TypeOperand::Variable(result), TypeOperand::Term(term))
                | (TypeOperand::Term(term), TypeOperand::Variable(result))
                    if *result == variable =>
                {
                    Some(self.inference.term(*term).clone())
                }
                _ => None,
            }
        })
    }

    /// Commit one type term.
    pub(super) fn commit_type_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        term: &TypeTerm,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        if !term.is_stable(self) {
            return None;
        }
        let origin = Origin::Node(source.into_global(module));
        let can_reduce_members = source.ty == dir::NodeType::Expression;
        let reduced =
            match self.reduce_committable_type_term(origin, module, term, can_reduce_members) {
                Ok(reduced) => reduced?,
                Err(error) => {
                    panic!("check type term failed to reduce during commit: {error:?}")
                }
            };
        if reduced != *term {
            return self.commit_type_term(module, output, environment, &reduced, source);
        }

        let ty = match term {
            TypeTerm::Type(ty) => return Some(*ty),
            TypeTerm::Literal(atom) => atom.to_type(),
            TypeTerm::Intrinsic => dir::Type::Intrinsic,
            TypeTerm::Parameter(parameter) => dir::Type::Parameter(*parameter),
            TypeTerm::This => dir::Type::This,
            TypeTerm::Form { form, payload } => {
                let value =
                    self.commit_type_operand(module, output, environment, *payload, source)?;
                let form = self.commit_form_term(module, output, environment, *form)?;

                dir::Type::Form(dir::FormType { form, value })
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => {
                let arguments =
                    self.commit_argument_terms(module, output, environment, arguments, source)?;

                dir::Type::Reference(dir::ReferenceType {
                    symbol: *symbol,
                    arguments,
                })
            }
            TypeTerm::Array { element } => {
                let element =
                    self.commit_type_operand(module, output, environment, *element, source)?;

                dir::Type::Array(dir::ArrayType { element })
            }
            TypeTerm::Member(member) => {
                let member = self.inference.term(*member).clone();
                let owner =
                    self.commit_type_operand(module, output, environment, member.owner, source)?;
                let arguments = self.commit_argument_terms(
                    module,
                    output,
                    environment,
                    &member.arguments,
                    source,
                )?;

                dir::Type::Member(dir::MemberType {
                    owner,
                    key: member.key,
                    arguments,
                })
            }
            TypeTerm::StaticValue { .. } => return None,
            TypeTerm::FixedArray { element, length } => {
                let element =
                    self.commit_type_operand(module, output, environment, *element, source)?;
                let count = self.commit_static_operand(module, output, environment, *length)?;

                dir::Type::FixedArray(dir::FixedArrayType { element, count })
            }
            TypeTerm::Slice { element } => {
                let element =
                    self.commit_type_operand(module, output, environment, *element, source)?;

                dir::Type::Slice(dir::SliceType { element })
            }
            TypeTerm::Tuple { form, elements } => dir::Type::Tuple(dir::TupleType {
                form: *form,
                elements: self.commit_tuple_elements(
                    module,
                    output,
                    environment,
                    elements,
                    source,
                )?,
            }),
            TypeTerm::Shape(shape) => {
                let members = self.inference.term(*shape).members.clone();

                dir::Type::Shape(self.commit_shape_type(
                    module,
                    output,
                    environment,
                    &members,
                    source,
                )?)
            }
            TypeTerm::Function(function) => dir::Type::Function(self.commit_function_type(
                module,
                output,
                environment,
                *function,
                source,
            )?),
            TypeTerm::Range {
                start,
                end,
                is_inclusive,
            } => dir::Type::Range(dir::RangeType {
                start: start.clone(),
                end: end.clone(),
                is_inclusive: *is_inclusive,
            }),
            TypeTerm::Union { elements } => dir::Type::Union(dir::UnionType {
                elements: self.commit_type_operands(
                    module,
                    output,
                    environment,
                    elements,
                    source,
                )?,
            }),
            TypeTerm::Intersection { elements } => dir::Type::Intersection(dir::IntersectionType {
                elements: self.commit_type_operands(
                    module,
                    output,
                    environment,
                    elements,
                    source,
                )?,
            }),
            TypeTerm::Operation(operation) => dir::Type::Operation(self.commit_type_operation(
                module,
                output,
                environment,
                *operation,
                source,
            )?),
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
            | TypeTerm::TaggedTemplate(_) => return None,
            TypeTerm::Dynamic { constraint } => {
                let constraint =
                    self.commit_type_operand(module, output, environment, *constraint, source)?;

                dir::Type::Dynamic(dir::DynamicType { constraint })
            }
            TypeTerm::Closure {
                function,
                environment: capture,
            } => {
                let function =
                    self.commit_type_operand(module, output, environment, *function, source)?;
                let environment =
                    self.commit_type_operand(module, output, environment, *capture, source)?;

                dir::Type::Closure(dir::ClosureType {
                    function,
                    environment,
                })
            }
        };

        Some(
            self.intern_type(module, output, ty, source)
                .into_global(module),
        )
    }

    /// Reduce one stable type term for commit without writing decisions.
    fn reduce_committable_type_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        can_reduce_members: bool,
    ) -> CompilerResult<Option<TypeTerm>> {
        match term {
            TypeTerm::Member(member) if can_reduce_members => {
                let member = self.inference.term(*member).clone();

                self.reduce_committable_member_term(origin, module, &member, can_reduce_members)
            }
            TypeTerm::Shape(shape) if can_reduce_members || self.shape_has_spread(*shape) => {
                Ok(self.reduce_type_term(origin, term)?.value)
            }
            _ => Ok(Some(term.clone())),
        }
    }

    /// Reduce one member projection for commit without selecting it.
    fn reduce_committable_member_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        member: &MemberTerm,
        can_reduce_members: bool,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(owner) = self.type_operand_term(member.owner)? else {
            return Ok(None);
        };
        let Some(owner) =
            self.reduce_committable_type_term(origin, module, &owner, can_reduce_members)?
        else {
            return Ok(None);
        };

        self.resolve_member_type(origin, module, &owner, &member.key, &member.arguments)
    }

    /// Return whether one shape still contains a spread member.
    fn shape_has_spread(&self, shape: TermId<ShapeTerm>) -> bool {
        self.inference
            .term(shape)
            .members
            .iter()
            .any(|member| matches!(member, ShapeMember::Spread { .. }))
    }

    /// Commit one static term.
    fn commit_static_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        term: &StaticTerm,
    ) -> Option<dir::GlobalStaticId> {
        let term = match term {
            StaticTerm::Static(static_id) => return Some(*static_id),
            StaticTerm::Literal(term) => term.clone(),
            StaticTerm::Parameter(parameter) => dir::StaticTerm::Parameter(*parameter),
            StaticTerm::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.commit_static_operand(module, output, environment, *element)
                    })
                    .collect::<Option<Vec<_>>>()?;

                dir::StaticTerm::Union { elements }
            }
            StaticTerm::Expression(_) => return None,
            StaticTerm::Member { .. }
            | StaticTerm::Layout(_)
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => {
                return None;
            }
        };

        Some(self.intern_static(module, output, term).into_global(module))
    }

    /// Commit one static operand.
    pub(in crate::check) fn commit_static_operand(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operand: StaticOperand,
    ) -> Option<dir::GlobalStaticId> {
        match operand {
            StaticOperand::Variable(variable) => {
                self.commit_static_variable(module, output, environment, variable)
            }
            StaticOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                self.commit_static_term(module, output, environment, &term)
            }
            StaticOperand::Static(static_id) => Some(static_id),
        }
    }

    /// Commit one variable as a static value inside one target module.
    fn commit_static_variable(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::GlobalStaticId> {
        match self.variable_static_solution_operand(variable)? {
            StaticOperand::Variable(_) => None,
            StaticOperand::Term(term) => {
                let term = self.inference.term(term).clone();

                self.commit_static_term(module, output, environment, &term)
            }
            StaticOperand::Static(static_id) => Some(static_id),
        }
    }

    /// Commit one memory form term.
    fn commit_form_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        form: TermId<FormTerm>,
    ) -> Option<dir::Form> {
        let form = self.inference.term(form).clone();
        let form = match form {
            FormTerm::Managed => dir::Form::Managed,
            FormTerm::Owned => dir::Form::Owned,
            FormTerm::Borrowed { lifetime, access } => {
                let lifetime = self.commit_static_operand(module, output, environment, lifetime)?;
                let access = self.commit_static_operand(module, output, environment, access)?;

                dir::Form::Borrowed { lifetime, access }
            }
            FormTerm::Raw => dir::Form::Raw,
            FormTerm::Placed { place } => {
                let place = self.commit_static_operand(module, output, environment, place)?;

                dir::Form::Placed { place }
            }
            FormTerm::Readonly => dir::Form::Readonly,
        };

        Some(form)
    }

    /// Return one solved type term for commit.
    fn commit_variable_term(&mut self, variable: VariableId) -> Option<TypeTerm> {
        match self.variable_type_solution_operand(variable)? {
            TypeOperand::Variable(_) => None,
            TypeOperand::Term(term) => Some(self.inference.term(term).clone()),
            TypeOperand::Type(ty) => Some(TypeTerm::Type(ty)),
        }
    }

    /// Commit generic parameter slots as parameter types.
    fn commit_generic_parameter_types(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        parameters: &[crate::check::GenericParameterId],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::GlobalTypeId>> {
        parameters
            .iter()
            .map(|parameter| {
                let ty = dir::Type::Parameter(*parameter);

                Some(
                    self.intern_type(module, output, ty, source)
                        .into_global(module),
                )
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
    ) -> Option<Vec<dir::GlobalTypeId>> {
        operands
            .iter()
            .map(|operand| self.commit_type_operand(module, output, environment, *operand, source))
            .collect()
    }

    /// Commit function parameter type variables.
    pub(in crate::check) fn commit_function_parameter_type_ids(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameter],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::GlobalTypeId>> {
        parameters
            .iter()
            .map(|parameter| {
                self.commit_type_operand(module, output, environment, parameter.ty, source)
            })
            .collect()
    }

    /// Commit function parameters.
    pub(in crate::check) fn commit_function_parameters(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        parameters: &[FunctionParameter],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::FunctionParameterType>> {
        parameters
            .iter()
            .map(|parameter| {
                let parameter = parameter;
                let ty = parameter.ty;
                let static_parameter = parameter.static_parameter;
                let is_optional = parameter.is_optional;
                let is_rest = parameter.is_rest;

                Some(dir::FunctionParameterType {
                    ty: self.commit_type_operand(module, output, environment, ty, source)?,
                    static_parameter,
                    is_optional,
                    is_rest,
                })
            })
            .collect()
    }

    /// Commit one resolved generic instance.
    pub(in crate::check) fn commit_generic_instance(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        node: dir::GlobalNodeIdAny,
        instance: &GenericInstance,
    ) -> Option<dir::LocalGenericInstanceId> {
        let source = node.local_id;
        let arguments =
            self.commit_argument_terms(module, output, environment, &instance.arguments, source)?;
        let template = output
            .generics
            .iter_templates()
            .find_map(|(id, template)| (template.owner == instance.owner).then_some(id))?;
        let instance = dir::GenericInstance::new(template, arguments);
        let instance_id = output
            .generics
            .find_instance(&instance)
            .unwrap_or_else(|| output.generics.push_instance(instance));

        output.generics.set_node_instance(node, instance_id);

        Some(instance_id)
    }

    /// Commit generic argument terms.
    pub(super) fn commit_argument_terms(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        arguments: &[GenericArgument],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::StaticArgument>> {
        arguments
            .iter()
            .map(|argument| {
                self.commit_argument_term(module, output, environment, argument, source)
            })
            .collect()
    }

    /// Commit one generic argument term.
    fn commit_argument_term(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        argument: &GenericArgument,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::StaticArgument> {
        let value = match argument {
            GenericArgument::Type(operand) => {
                let ty = self.commit_type_operand(module, output, environment, *operand, source)?;

                self.commit_type_argument(module, output, ty).value
            }
            GenericArgument::Static(operand) => {
                self.commit_static_operand(module, output, environment, *operand)?
            }
            GenericArgument::AssociatedType { name, value } => {
                let ty = self.commit_type_operand(module, output, environment, *value, source)?;
                let value = self.commit_type_argument(module, output, ty).value;

                return Some(dir::StaticArgument {
                    name: Some(*name),
                    value,
                });
            }
            GenericArgument::AssociatedConst { name, value } => {
                let value = self.commit_static_operand(module, output, environment, *value)?;

                return Some(dir::StaticArgument {
                    name: Some(*name),
                    value,
                });
            }
            GenericArgument::TypeOrStatic { .. }
            | GenericArgument::SpreadType(_)
            | GenericArgument::SpreadStatic(_)
            | GenericArgument::SpreadTypeOrStatic { .. } => return None,
        };

        Some(dir::StaticArgument::value(value))
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
    ) -> Option<Vec<dir::TypeElement>> {
        elements
            .iter()
            .map(|element| {
                let element = element;
                let label = element.label;
                let ty = element.ty;
                let is_optional = element.is_optional;
                let is_readonly = element.is_readonly;
                let is_rest = element.is_rest;

                Some(dir::TypeElement {
                    label,
                    ty: self.commit_type_operand(module, output, environment, ty, source)?,
                    is_optional,
                    is_readonly,
                    is_rest,
                })
            })
            .collect()
    }

    /// Commit one structural shape type.
    fn commit_shape_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        members: &[ShapeMember],
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::ShapeType> {
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
                    ty: self.commit_type_operand(module, output, environment, ty, source)?,
                    is_optional,
                    is_readonly,
                }),
                ShapeMember::Spread { .. } => return None,
                ShapeMember::CallSignature { ty } => {
                    call_signatures.push(self.commit_type_operand(
                        module,
                        output,
                        environment,
                        ty,
                        source,
                    )?);
                }
                ShapeMember::ConstructSignature { ty } => {
                    construct_signatures.push(self.commit_type_operand(
                        module,
                        output,
                        environment,
                        ty,
                        source,
                    )?);
                }
                ShapeMember::IndexSignature {
                    name,
                    key_type,
                    value_type,
                    is_optional,
                    is_readonly,
                } => index_signatures.push(dir::TypeIndexSignature {
                    name,
                    key_type: self.commit_type_operand(
                        module,
                        output,
                        environment,
                        key_type,
                        source,
                    )?,
                    value_type: self.commit_type_operand(
                        module,
                        output,
                        environment,
                        value_type,
                        source,
                    )?,
                    is_optional,
                    is_readonly,
                }),
            }
        }

        Some(dir::ShapeType {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        })
    }

    /// Commit one function type.
    fn commit_function_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        function: TermId<FunctionTerm>,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::FunctionType> {
        let function = self.inference.term(function).clone();

        Some(dir::FunctionType {
            asynchrony: function.asynchrony,
            generic_parameters: self.commit_generic_parameter_types(
                module,
                output,
                &function.generic_parameters,
                source,
            )?,
            this_parameter: function.this_parameter.and_then(|parameter| {
                self.commit_type_operand(module, output, environment, parameter, source)
            }),
            parameters: self.commit_function_parameters(
                module,
                output,
                environment,
                &function.parameters,
                source,
            )?,
            return_type: function.return_type.and_then(|return_type| {
                self.commit_type_operand(module, output, environment, return_type, source)
            }),
            is_generator: function.is_generator,
        })
    }

    /// Commit one type operation.
    fn commit_type_operation(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        operation: TermId<TypeOperationTerm>,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::TypeOperation> {
        let operation = self.inference.term(operation).clone();
        let operation = match operation {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => dir::TypeOperation::Conditional(dir::ConditionalType {
                left: self.commit_type_operand(module, output, environment, left, source)?,
                right: self.commit_type_operand(module, output, environment, right, source)?,
                then_type: self.commit_type_operand(
                    module,
                    output,
                    environment,
                    then_type,
                    source,
                )?,
                else_type: self.commit_type_operand(
                    module,
                    output,
                    environment,
                    else_type,
                    source,
                )?,
            }),
            TypeOperationTerm::Index { left, index } => dir::TypeOperation::Index(dir::IndexType {
                left: self.commit_type_operand(module, output, environment, left, source)?,
                index: self.commit_type_operand(module, output, environment, index, source)?,
            }),
            TypeOperationTerm::TemplateLiteral { strings, spans } => {
                dir::TypeOperation::TemplateLiteral(dir::TemplateLiteralType {
                    strings,
                    spans: self.commit_type_operands(
                        module,
                        output,
                        environment,
                        &spans,
                        source,
                    )?,
                })
            }
            TypeOperationTerm::Infer { name, constraint } => {
                dir::TypeOperation::Infer(dir::InferType {
                    name,
                    constraint: constraint.and_then(|constraint| {
                        self.commit_type_operand(module, output, environment, constraint, source)
                    }),
                })
            }
            TypeOperationTerm::KeyOf { target } => dir::TypeOperation::KeyOf(dir::UnaryType {
                target: self.commit_type_operand(module, output, environment, target, source)?,
            }),
            TypeOperationTerm::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let name = parameter.name;
                let symbol = parameter.symbol;
                let constraint = parameter.constraint;
                let key_remap = parameter.key_remap;

                dir::TypeOperation::Mapped(dir::MappedType {
                    parameter: dir::MappedTypeParameter {
                        name,
                        symbol,
                        constraint: self.commit_type_operand(
                            module,
                            output,
                            environment,
                            constraint,
                            source,
                        )?,
                        key_remap: key_remap.and_then(|key_remap| {
                            self.commit_type_operand(module, output, environment, key_remap, source)
                        }),
                    },
                    modifiers,
                    value: self.commit_type_operand(module, output, environment, value, source)?,
                })
            }
            TypeOperationTerm::StringMapping { mapping, argument } => {
                dir::TypeOperation::StringMapping {
                    mapping,
                    target: self.commit_type_operand(
                        module,
                        output,
                        environment,
                        argument,
                        source,
                    )?,
                }
            }
            TypeOperationTerm::BestCommon { .. }
            | TypeOperationTerm::Widen { .. }
            | TypeOperationTerm::Exclude { .. }
            | TypeOperationTerm::Intrinsic { .. } => return None,
        };

        Some(operation)
    }
}
