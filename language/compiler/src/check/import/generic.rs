use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, GenericParameter, GenericParameterBinding, GenericParameterId, GenericTemplate,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import one committed generic template id.
    pub(in crate::check) fn import_generic_template_id(
        &mut self,
        module: ModuleId,
        template: dir::GlobalGenericTemplateId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        if self.is_component_module(template.module_id) {
            return Ok(());
        }

        if self.inference.generic_template(template).is_some() {
            if let Some(symbol) = symbol {
                self.inference
                    .upsert_symbol_generic_template(template, symbol)?;
            }

            return Ok(());
        }

        let committed = self
            .dependency(template.module_id)
            .generics
            .get_template(template.local_id)
            .clone();
        let source = committed.source;
        let parent = committed
            .parent
            .map(|parent| parent.into_global(template.module_id));
        if let Some(parent) = parent {
            self.import_generic_template_id(module, parent, None)?;
        }

        let dependency = self.dependency(template.module_id);
        let parameters = committed
            .parameters
            .iter()
            .map(|parameter_id| {
                (
                    (*parameter_id).into_global(template.module_id),
                    *dependency.generics.get_parameter(*parameter_id),
                )
            })
            .collect::<Vec<_>>();
        let template_entry = GenericTemplate::new(template, source, parent);

        self.inference
            .insert_generic_template(template_entry, symbol);
        for (parameter_id, parameter) in parameters {
            let generic =
                self.import_generic_parameter(module, template, parameter_id, parameter)?;

            self.inference.insert_generic_parameter(generic);
        }

        Ok(())
    }

    /// Import one committed generic parameter id.
    pub(in crate::check) fn import_generic_parameter_id(
        &mut self,
        module: ModuleId,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<GenericParameterId> {
        if self.is_component_module(parameter.module_id) {
            return Ok(parameter);
        }

        if self.inference.generic_parameter(parameter).is_some() {
            return Ok(parameter);
        }

        let dependency = self.dependency(parameter.module_id);
        let generic = dependency.generics.get_parameter(parameter.local_id);
        let template = generic.template().into_global(parameter.module_id);

        self.import_generic_template_id(module, template, None)?;
        if self.inference.generic_parameter(parameter).is_none() {
            return Err(CompilerError::Internal {
                message: format!("dependency generic parameter {parameter:?} has no parameter"),
            });
        }

        Ok(parameter)
    }

    /// Import one generic parameter.
    fn import_generic_parameter(
        &mut self,
        module: ModuleId,
        template: dir::GlobalGenericTemplateId,
        id: dir::GlobalGenericParameterId,
        parameter: dir::GenericParameterBinding,
    ) -> CompilerResult<GenericParameterBinding> {
        let header = GenericParameter::new(id, template, parameter.key(), parameter.origin());

        match parameter {
            dir::GenericParameterBinding::Type {
                variance,
                constraint,
                default,
                ..
            } => {
                let constraint = constraint
                    .map(|id| self.import_type_operand(module, id))
                    .transpose()?;
                let default = default
                    .map(|id| self.import_type_operand(module, id))
                    .transpose()?;

                Ok(GenericParameterBinding::r#type(
                    header, variance, constraint, default,
                ))
            }
            dir::GenericParameterBinding::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => {
                let constraint = constraint
                    .map(|id| self.import_type_operand(module, id))
                    .transpose()?;
                let default = default
                    .map(|id| self.import_type_operand(module, id))
                    .transpose()?;

                Ok(GenericParameterBinding::variadic_type(
                    header, variance, constraint, default,
                ))
            }
            dir::GenericParameterBinding::Static {
                constraint,
                default,
                ..
            } => {
                let constraint = constraint
                    .map(|id| self.import_type_operand(module, id))
                    .transpose()?;
                let default = default
                    .map(|id| self.import_static_operand(module, id))
                    .transpose()?;

                Ok(GenericParameterBinding::r#static(
                    header, constraint, default,
                ))
            }
            dir::GenericParameterBinding::VariadicStatic {
                constraint,
                default,
                ..
            } => {
                let constraint = constraint
                    .map(|id| self.import_type_operand(module, id))
                    .transpose()?;
                let default = default
                    .map(|id| self.import_static_operand(module, id))
                    .transpose()?;

                Ok(GenericParameterBinding::variadic_static(
                    header, constraint, default,
                ))
            }
        }
    }
}
