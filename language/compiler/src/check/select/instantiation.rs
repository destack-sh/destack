use crate::check::{
    Answer, BodyState, CheckState, Decision, DecisionKind, Dependency, GenericParameterId,
    GenericTemplateId, Origin, TypeSubstitution, VariableRole, Widening, answer,
};
use crate::{CompilerError, CompilerResult};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

/// Literal inference selected for one generic application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TypeArgumentInference<'a> {
    /// Preserve exact type arguments.
    Exact,
    /// Infer arguments for one callable signature.
    Callable {
        /// The callable parameter types before substitution.
        parameters: &'a [dir::FunctionParameterType],
        /// The callable return type before substitution.
        return_type: Option<dir::GlobalTypeId>,
    },
}

impl CheckState<'_> {
    /// Bind explicit arguments and declared defaults to one template.
    pub(in crate::check) fn bind_explicit_arguments(
        &mut self,
        module: ModuleId,
        template: GenericTemplateId,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        let parameters = self.generic_template_parameters(template)?;
        if written.len() > self.writable_parameter_count(&parameters) {
            return Ok(None);
        }

        // bind writable parameters and fill omitted defaults
        let mut substitution = TypeSubstitution::default();
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            let binding = *self.require_generic_parameter(parameter)?;
            let is_explicit = matches!(binding.origin, dir::GenericParameterOrigin::Explicit);
            if binding.is_writable() && cursor < written.len() {
                substitution.bind(parameter, written[cursor])?;
                cursor += 1;

                continue;
            }

            // evaluate defaults against the application built so far
            let default = binding
                .default
                .map(|default| self.substitute_type(module, default, &substitution))
                .transpose()?;
            if let Some(default) = default {
                substitution.bind(parameter, default)?;

                continue;
            }

            if is_explicit {
                return Ok(None);
            }
        }

        Ok(Some(substitution))
    }

    /// Instantiate one parameter list, opening every omitted parameter.
    pub(in crate::check) fn instantiate_parameters(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        mut substitution: TypeSubstitution,
        inference: TypeArgumentInference<'_>,
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        let writable = parameters
            .iter()
            .filter(|parameter| {
                substitution.argument(**parameter).is_none()
                    && self
                        .generic_parameter(**parameter)
                        .is_some_and(dir::GenericParameterBinding::is_writable)
            })
            .count();
        if written.len() > writable {
            return Ok(Answer::Ready(None));
        }

        // bind written parameters and open omitted inference parameters
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            if substitution.argument(parameter).is_some() {
                continue;
            }
            let binding = *self.require_generic_parameter(parameter)?;
            if binding.is_writable() && cursor < written.len() {
                substitution.bind(parameter, written[cursor])?;
                cursor += 1;

                continue;
            }

            // reuse a parameter opened earlier at this typing position
            let origin_id = self.solver.intern_origin(origin);
            if let Some(existing) =
                self.solver
                    .instantiation(origin_id, parameter, substitution.receiver)
            {
                let argument = self.variable_type(existing)?;
                substitution.bind(parameter, argument)?;

                continue;
            }

            // open one inference variable for the omitted parameter
            let widening =
                answer!(self.type_argument_widening(origin, parameter, binding, inference,)?);
            let variable =
                self.allocate_variable(origin, widening, VariableRole::Instantiation { parameter });
            self.solver
                .record_instantiation(origin_id, parameter, substitution.receiver, variable);

            // retain the declared default for dry inference
            if let Some(default) = binding.default {
                let default = self.substitute_type(origin.module(), default, &substitution)?;
                self.set_variable_default(variable, default);
            }

            let argument = self.variable_type(variable)?;
            substitution.bind(parameter, argument)?;
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Return the literal widening policy for one inferred type argument.
    fn type_argument_widening(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
        binding: dir::GenericParameterBinding,
        inference: TypeArgumentInference<'_>,
    ) -> CompilerResult<Answer<Widening>> {
        let TypeArgumentInference::Callable {
            parameters,
            return_type,
        } = inference
        else {
            return Ok(Answer::Ready(Widening::Never));
        };
        let mut preserves_literals = binding.is_const || binding.is_comptime();
        for bound in self.declared_parameter_bounds(parameter)? {
            if answer!(self.scalar_families(origin, bound)?).is_some() {
                preserves_literals = true;

                break;
            }
        }
        if preserves_literals {
            return Ok(Answer::Ready(Widening::Never));
        }

        // preserve one direct input candidate only when callers observe it directly
        let mut is_direct_input = false;
        for parameter in parameters {
            if self.exposes_type(parameter.ty, binding.ty)? {
                is_direct_input = true;

                break;
            }
        }
        let is_direct_output = match return_type {
            Some(return_type) => self.exposes_type(return_type, binding.ty)?,
            None => false,
        };
        let widening = match (is_direct_input, is_direct_output) {
            (false, _) => Widening::Aggregate,
            (true, true) => Widening::Multiple,
            (true, false) => Widening::Always,
        };

        Ok(Answer::Ready(widening))
    }

    /// Return whether a type exposes another type through transparent alternatives.
    pub(in crate::check) fn exposes_type(
        &self,
        ty: dir::GlobalTypeId,
        exposed: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(&[ty]);
        let mut visited = SmallVec::<[dir::GlobalTypeId; 8]>::new();

        // follow only unions, intersections, and conditional result alternatives
        while let Some(ty) = pending.pop() {
            if ty == exposed {
                return Ok(true);
            }
            if visited.contains(&ty) {
                continue;
            }
            visited.push(ty);

            match self.ty(ty)? {
                dir::Type::Form(form) => pending.push(form.value),
                dir::Type::Union(union) => {
                    pending.extend(self.type_ids(ty.module_id, union.elements)?.iter().copied());
                }
                dir::Type::Intersection(intersection) => {
                    pending.extend(
                        self.type_ids(ty.module_id, intersection.elements)?
                            .iter()
                            .copied(),
                    );
                }
                dir::Type::Operation(_) => {
                    if let Some(dir::TypeOperation::Conditional(conditional)) =
                        self.operation_head(ty)?
                    {
                        pending.push(conditional.then_type);
                        pending.push(conditional.else_type);
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }
}

impl BodyState<'_, '_> {
    /// Select one explicit instantiation once its target name decides.
    pub(in crate::check) fn select_instantiation(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let source = node.local_id.into_any();
        let node = node.into_any();
        let origin = self.node_site(node)?.origin();

        // read the decided target name
        let left_node = left.into_global_any(module);
        let name = self
            .resolutions(left_node.module_id)
            .name_resolution(left_node)
            .cloned();
        let symbol = match (self.decision_kind(left_node), name) {
            (Some(DecisionKind::Name), Some(resolution)) => match resolution.symbols() {
                [symbol] => *symbol,
                _ => {
                    let Some(path) = self.module(module).view().tree().reference_path(left) else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "overloaded instantiation target {left_node:?} has no reference path"
                            ),
                        });
                    };
                    self.report_ambiguous_reference(module, left.into_any(), &path);
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                }
            },
            (Some(DecisionKind::Rejected), _) => {
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            }
            (Some(other), _) => {
                return Err(CompilerError::Internal {
                    message: format!("instantiation target {left_node:?} decided as {other:?}"),
                });
            }
            // references decide during the walk
            (None, _) => {
                return Err(CompilerError::Internal {
                    message: format!("instantiation target {left_node:?} has no walk decision"),
                });
            }
        };

        // collect the written argument types
        let mut applied = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for argument in arguments {
            let argument = argument.into_global_any(module);
            let ty = self.require_node_type(argument)?;
            applied.push(ty);
        }

        // read the selected declaration template
        let template = self.symbol_template(symbol)?;
        if template.is_none() && !applied.is_empty() {
            let name = self.format_symbol(symbol);
            self.report_wrong_generic_arity(module, source, name, 0, applied.len());
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        }

        // specialize the selected value type by the applied arguments
        let Some(declared) = self.symbol_type_maybe(symbol) else {
            return Ok(Answer::pending([Dependency::SymbolType(symbol)]));
        };
        let specialized = match template {
            Some(template) => {
                let parameters = self.generic_template_parameters(template)?;
                let Some(substitution) =
                    self.bind_explicit_arguments(module, template, &applied)?
                else {
                    let name = self.format_symbol(symbol);
                    let written_count = self.writable_parameter_count(&parameters);
                    self.report_wrong_generic_arity(
                        module,
                        source,
                        name,
                        written_count,
                        applied.len(),
                    );
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                };

                // register constraints determined by this application
                for constraint in
                    self.substitute_application_constraints(origin, template, &substitution)?
                {
                    self.check.push_constraint(constraint);
                }

                match self.ty(declared)? {
                    dir::Type::Reference(reference) if reference.symbol == symbol => {
                        let arguments = substitution.arguments().collect::<SmallVec<[_; 4]>>();
                        let arguments = self.intern_type_ids(module, &arguments)?;

                        self.intern_type(
                            module,
                            dir::Type::Application(dir::GenericApplication { symbol, arguments }),
                        )?
                    }
                    _ => self.substitute_type(module, declared, &substitution)?,
                }
            }
            None => declared,
        };

        let arguments = self.symbol_generic_argument_bindings(symbol, &applied)?;
        let resolution = dir::InstantiationResolution::new(symbol, arguments);
        self.commit_decision(node, Decision::Instantiation(resolution))?;
        self.commit_node_type(node, specialized)?;

        Ok(Answer::Ready(()))
    }
}
