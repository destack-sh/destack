use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CheckState, GenericParameterId, GenericTemplateId, Origin, TypeSubstitution,
    VariableRole,
};
use crate::{CompilerError, CompilerResult};

/// Literal inference selected for one generic application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum TypeArgumentInference<'a> {
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
    /// Interpret one written argument for a parameter slot.
    ///
    /// A const slot resolves a value binding to its static value; a type slot
    /// rejects value bindings, refusing the candidate.
    fn slot_written_argument(
        &mut self,
        binding: &dir::GenericParameterBinding,
        written: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // pass every argument except a symbolic value binding through
        let dir::Type::Reference(reference) = self.ty(written)? else {
            return Ok(Some(written));
        };
        if !self.symbol_kind(reference.symbol)?.is_binding() {
            return Ok(Some(written));
        }

        // resolve a const slot's binding to its committed static value
        if binding.is_const {
            if let Some(value) = self.static_value(reference.symbol) {
                return Ok(Some(value));
            }

            // keep the reference symbolic while declaring
            if self.is_declaring() {
                return Ok(Some(written));
            }
        }

        Ok(None)
    }

    /// Bind explicit arguments and declared defaults to one template.
    pub(in crate::sema) fn bind_explicit_arguments(
        &mut self,
        _module: ModuleId,
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
                // interpret a value binding argument by the slot's kind
                let Some(argument) = self.slot_written_argument(&binding, written[cursor])? else {
                    return Ok(None);
                };
                substitution.bind(parameter, argument)?;
                cursor += 1;

                continue;
            }

            // evaluate defaults against the application built so far
            let default = binding
                .default
                .map(|default| self.substitute_type(default, &substitution))
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
    pub(in crate::sema) fn instantiate_parameters(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        mut substitution: TypeSubstitution,
        _inference: TypeArgumentInference<'_>,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        self.counters.instantiations += 1;
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
            return Ok(None);
        }

        // bind written parameters and open omitted inference parameters
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            if substitution.argument(parameter).is_some() {
                continue;
            }
            let binding = *self.require_generic_parameter(parameter)?;
            if binding.is_writable() && cursor < written.len() {
                // interpret a value binding argument by the slot's kind
                let Some(argument) = self.slot_written_argument(&binding, written[cursor])? else {
                    return Ok(None);
                };
                substitution.bind(parameter, argument)?;
                cursor += 1;

                continue;
            }

            // reuse a parameter opened earlier at this typing position
            let origin_id = self.infer.intern_origin(origin);
            if let Some(existing) = self.infer.instantiation(origin_id, parameter) {
                let argument = self.variable_type(existing)?;
                substitution.bind(parameter, argument)?;

                continue;
            }

            // open one inference variable for the omitted parameter
            let variable = self.open_variable(origin, VariableRole::Instantiation { parameter });

            // record the instantiation while the site claims its typing position
            self.infer
                .insert_instantiation(origin_id, parameter, variable);

            // keep the declared default for dry inference
            if let Some(default) = binding.default {
                let default = self.substitute_type(default, &substitution)?;
                self.set_variable_default(variable, default);
            }

            let argument = self.variable_type(variable)?;
            substitution.bind(parameter, argument)?;
        }

        Ok(Some(substitution))
    }

    /// Return whether a type exposes another type through transparent alternatives.
    pub(in crate::sema) fn has_exposed_type(
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
    pub(in crate::sema) fn select_instantiation(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let source = node.local_id.into_any();
        let node = node.into_any();
        let origin = self.visit_site(node)?.origin();

        // decide the target reference at its first visit
        let left_node = left.into_global_any(module);
        let symbol = match self.decide_reference(left_node)? {
            Some(resolution) => match resolution.symbols() {
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
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(());
                }
            },
            None => match self.decision(left_node).cloned() {
                // reject the instantiation with its rejected target
                Some(dir::Decision::Rejected | dir::Decision::Poisoned) => {
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(());
                }
                // fail loudly on any other decision, which names no instantiation target
                Some(other) => {
                    return Err(CompilerError::Internal {
                        message: format!("instantiation target {left_node:?} decided as {other:?}"),
                    });
                }
                // references decide during the walk
                None => {
                    return Err(CompilerError::Internal {
                        message: format!("instantiation target {left_node:?} has no walk decision"),
                    });
                }
            },
        };

        // type the written arguments at their first visit
        self.walk_body_generic_arguments(module, arguments)?;

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
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        }

        // specialize the selected value type by the applied arguments
        let declared = self.symbol_type(symbol)?;
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
                    self.commit_decision(node, dir::Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(());
                };

                // push constraints determined by this application
                for constraint in
                    self.substitute_application_constraints(origin, template, &substitution)?
                {
                    self.check.push_relation(constraint)?;
                }

                match self.ty(declared)? {
                    dir::Type::Reference(reference) if reference.symbol == symbol => {
                        let arguments = substitution.arguments().collect::<SmallVec<[_; 4]>>();
                        let arguments = self.intern_type_ids(&arguments)?;

                        self.intern_type(dir::Type::Application(dir::GenericApplication {
                            symbol,
                            arguments,
                        }))?
                    }
                    _ => self.substitute_type(declared, &substitution)?,
                }
            }
            None => declared,
        };

        // build the applied callable value with its selected arguments
        let arguments = self.symbol_generic_argument_bindings(symbol, &applied)?;
        let value = dir::FunctionValue {
            target: dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: None,
                    selection: dir::Selection::new(symbol, arguments),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            callable_type: specialized,
        };
        self.commit_decision(
            node,
            dir::Decision::Function(dir::OperationResolution::One(value)),
        )?;
        self.commit_node_type(node, specialized)?;

        Ok(())
    }
}
