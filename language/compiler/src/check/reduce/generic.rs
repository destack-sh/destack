use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    BoundMode, CheckState, GenericInductionParameter, GenericParameterId, GenericTemplateId,
    Origin, Relation, TypeSubstitution, Widening,
};

/// Position of one written generic application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum GenericPosition {
    /// A written annotation: omitted parameters fill from defaults.
    Annotation,
    /// An inference site: omitted parameters open as variables.
    Inference,
}

/// One generic instantiation opened at a source site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct Opening {
    /// The source site that opened the variables.
    pub(in crate::check) site: dir::GlobalNodeIdAny,
    /// The leading opened parameter, distinguishing lists at one site.
    pub(in crate::check) parameter: dir::GlobalGenericParameterId,
}

impl CheckState<'_> {
    /// Return the inference widening policy for one generic parameter.
    fn generic_parameter_widening(&self, id: GenericParameterId) -> Widening {
        let Some(parameter) = self.generic_parameter(id) else {
            return Widening::Preserve;
        };

        if parameter.is_const || parameter.is_comptime {
            Widening::Preserve
        } else {
            Widening::Widen
        }
    }

    /// Return one generic parameter's default after earlier arguments apply.
    pub(in crate::check) fn generic_parameter_default(
        &mut self,
        module: ModuleId,
        parameter: GenericParameterId,
        parameters: &[GenericParameterId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(default) = self
            .generic_parameter(parameter)
            .and_then(|binding| binding.default)
        else {
            return Ok(None);
        };
        if parameters.is_empty() {
            return Ok(Some(default));
        }

        // apply earlier generic arguments before reading the default
        let substitution = TypeSubstitution {
            parameters: parameters.iter().copied().collect(),
            arguments: arguments.iter().copied().collect(),
            receiver: None,
        };
        let default = self.substitute_type(module, default, &substitution)?;

        Ok(Some(default))
    }

    /// Apply written and inferred arguments to one template.
    pub(in crate::check) fn apply_template_arguments(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        let parameters = self.generic_template_parameters(template);

        self.instantiate_generic_parameters(
            origin,
            &parameters,
            written,
            GenericPosition::Inference,
        )
    }

    /// Instantiate one parameter list through a solver-stable opening.
    pub(in crate::check) fn instantiate_at_opening(
        &mut self,
        origin: Origin,
        opening: Opening,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // reuse variables opened by an earlier poll
        if let Some(existing) = self.solver.opened_substitution(opening) {
            return Ok(Some(existing.clone()));
        }

        // open omitted parameters once
        let substitution = self.instantiate_generic_parameters(
            origin,
            parameters,
            written,
            GenericPosition::Inference,
        )?;
        if let Some(substitution) = &substitution {
            self.solver
                .insert_opened_substitution(opening, substitution.clone());
        }

        Ok(substitution)
    }

    /// Instantiate one parameter list from written arguments.
    ///
    /// Written arguments bind explicit parameters positionally.
    /// Annotations fill omitted parameters from their defaults, while
    /// inference sites open them as variables.
    pub(in crate::check) fn instantiate_generic_parameters(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        position: GenericPosition,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        if written.len() > self.written_parameter_count(parameters) {
            return Ok(None);
        }
        let source = self.origin_source_node(origin)?;
        let source_node = source.into_global(origin.module());

        // open every slot first so bounds may reference any parameter
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut opened = SmallVec::<[Option<dir::TypeVariableId>; 4]>::new();
        let mut cursor = 0;
        for (index, parameter) in parameters.iter().copied().enumerate() {
            let is_explicit = self.generic_parameter(parameter).is_some_and(|binding| {
                matches!(binding.origin, dir::GenericParameterOrigin::Explicit)
            });
            if is_explicit && cursor < written.len() {
                arguments.push(written[cursor]);
                cursor += 1;
                opened.push(None);
                continue;
            }

            // fill omitted annotation arguments from declared defaults
            if position == GenericPosition::Annotation {
                let default = self.generic_parameter_default(
                    origin.module(),
                    parameter,
                    &parameters[..index],
                    &arguments,
                )?;
                if let Some(default) = default {
                    arguments.push(default);
                    opened.push(None);
                    continue;
                }

                // reject omitted explicit arguments without a default
                if is_explicit {
                    return Ok(None);
                }
            }
            let widening = self.generic_parameter_widening(parameter);
            let variable = self.allocate_variable(origin.module(), origin, widening);
            arguments.push(self.variable_type(variable)?);
            opened.push(Some(variable));
        }
        let substitution = TypeSubstitution {
            parameters: parameters.iter().copied().collect(),
            arguments: arguments.clone(),
            receiver: None,
        };

        // bound each opened variable under the full substitution
        for (index, parameter) in parameters.iter().copied().enumerate() {
            let Some(variable) = opened[index] else {
                continue;
            };
            let Some(binding) = self.generic_parameter(parameter) else {
                continue;
            };
            let (constraint, origin_kind, is_comptime) =
                (binding.constraint, binding.origin, binding.is_comptime);
            let constraint = constraint
                .map(|constraint| self.substitute_type(origin.module(), constraint, &substitution))
                .transpose()?;
            if let Some(constraint) = constraint {
                self.push_upper_bound(
                    variable,
                    source_node,
                    constraint,
                    Relation::Satisfies,
                    BoundMode::Strong,
                )?;
            }

            // record defaults as weak solve bounds
            if let Some(default) = self.generic_parameter_default(
                origin.module(),
                parameter,
                &substitution.parameters[..index],
                &substitution.arguments[..index],
            )? {
                self.set_variable_default(variable, default)?;
            }

            // re-generalize induced parameters through the induction scan
            if position == GenericPosition::Annotation
                && let dir::GenericParameterOrigin::Induced(induction) = origin_kind
            {
                let name_prefix = match induction {
                    dir::GenericParameterInduction::Form => "L",
                    dir::GenericParameterInduction::Comptime => "C",
                    _ => "T",
                };
                self.generics.insert_induction(
                    variable,
                    GenericInductionParameter {
                        name_prefix,
                        constraint,
                        is_comptime,
                        induction,
                    },
                )?;
            }
        }

        Ok(Some(substitution))
    }

    /// Return how many parameters accept written arguments.
    pub(in crate::check) fn written_parameter_count(
        &self,
        parameters: &[GenericParameterId],
    ) -> usize {
        parameters
            .iter()
            .filter(|parameter| {
                self.generic_parameter(**parameter).is_some_and(|binding| {
                    matches!(binding.origin, dir::GenericParameterOrigin::Explicit)
                })
            })
            .count()
    }
}
