use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, GenericParameterId, GenericTemplateId, Origin, TypeSubstitution, Widening,
};

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

    /// Apply written and defaulted arguments to one template.
    pub(in crate::check) fn apply_template_arguments(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        let parameters = self.generic_template_parameters(template);

        self.apply_generic_parameters(origin, &parameters, written)
    }

    /// Apply written and defaulted arguments to one parameter list.
    pub(in crate::check) fn apply_generic_parameters(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        if written.len() > parameters.len() {
            return Ok(None);
        }
        let mut arguments = SmallVec::new();

        // apply written arguments before filling defaults
        for (index, parameter) in parameters.iter().copied().enumerate() {
            let ty = match written.get(index).copied() {
                Some(written) => written,
                None => {
                    let Some(default) = self.generic_parameter_default(
                        origin.module(),
                        parameter,
                        &parameters[..index],
                        &arguments,
                    )?
                    else {
                        return Ok(None);
                    };

                    default
                }
            };

            arguments.push(ty);
        }

        Ok(Some(TypeSubstitution {
            parameters: parameters.iter().copied().collect(),
            arguments,
            receiver: None,
        }))
    }

    /// Open omitted generic arguments as inference variables.
    pub(in crate::check) fn open_generic_parameters(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        if written.len() > parameters.len() {
            return Ok(None);
        }
        let source = self.origin_source_node(origin)?;
        let source_node = source.into_global(origin.module());
        let mut arguments = SmallVec::new();

        // apply written arguments before opening inference variables
        for (index, parameter) in parameters.iter().copied().enumerate() {
            let ty = match written.get(index).copied() {
                Some(written) => written,
                None => {
                    let widening = self.generic_parameter_widening(parameter);
                    let variable = self.allocate_variable(origin.module(), origin, widening);
                    let ty = self.variable_type(variable)?;

                    // add declared bounds as upper bounds
                    let constraint = self
                        .generic_parameter(parameter)
                        .and_then(|binding| binding.constraint);
                    if let Some(constraint) = constraint {
                        let substitution = TypeSubstitution {
                            parameters: parameters[..index].iter().copied().collect(),
                            arguments: arguments.iter().copied().collect(),
                            receiver: None,
                        };
                        let constraint =
                            self.substitute_type(origin.module(), constraint, &substitution)?;
                        self.push_upper_bound(variable, source_node, constraint)?;
                    }
                    if let Some(default) = self.generic_parameter_default(
                        origin.module(),
                        parameter,
                        &parameters[..index],
                        &arguments,
                    )? {
                        self.set_variable_default(variable, default)?;
                    }

                    ty
                }
            };

            arguments.push(ty);
        }

        Ok(Some(TypeSubstitution {
            parameters: parameters.iter().copied().collect(),
            arguments,
            receiver: None,
        }))
    }
}
