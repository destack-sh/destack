use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, GenericParameterBinding, TypeOperand};
use crate::{CompilerError, CompilerResult};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit generic templates and parameter bindings.
    pub(super) fn commit_generic_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::GenericSegment> {
        let mut table = dir::GenericSegment::new(module);

        // commit templates in allocated id order
        let mut templates = self
            .inference
            .generic_templates()
            .filter(|(template, _)| template.module_id == module)
            .map(|(template, value)| (template, value.clone()))
            .collect::<Vec<_>>();
        templates.sort_by_key(|(template, _)| template.local_id);

        for (template_id, template) in templates {
            let template = dir::GenericTemplate {
                source: template.source,
                parent: template.parent.map(|parent| parent.local_id),
                parameters: template
                    .parameters
                    .iter()
                    .map(|parameter| parameter.local_id)
                    .collect(),
            };
            let committed_id = table.push_template(template);

            // validate stable template id allocation
            if committed_id != template_id.local_id {
                return Err(CompilerError::Internal {
                    message: format!(
                        "generic template {template_id:?} committed as {committed_id:?}"
                    ),
                });
            }
        }

        // commit parameters in allocated id order
        let mut parameters = self
            .inference
            .generic_parameters()
            .filter(|(_, generic)| generic.parameter().id.module_id == module)
            .map(|(parameter_id, generic)| (parameter_id, *generic))
            .collect::<Vec<_>>();
        parameters.sort_by_key(|(parameter_id, _)| parameter_id.local_id);

        for (parameter_id, generic) in parameters {
            let template = generic.parameter().template.local_id;
            let parameter =
                self.commit_generic_parameter(module, output, environment, template, generic)?;
            let committed_id = table.push_parameter(parameter);

            // validate stable parameter id allocation
            if committed_id != parameter_id.local_id {
                return Err(CompilerError::Internal {
                    message: format!(
                        "generic parameter {parameter_id:?} committed as {committed_id:?}"
                    ),
                });
            }
        }

        let mut instances = self
            .inference
            .generic_instances()
            .filter(|(key, _)| key.source.module_id == module)
            .map(|(key, instance)| (key, instance.clone()))
            .collect::<Vec<_>>();
        instances.sort_by_key(|(key, _)| key.source.local_id.id);

        for (key, instance) in instances {
            self.commit_generic_instance_in_table(
                module,
                output,
                environment,
                &mut table,
                key.source.local_id,
                &instance,
            )?;
        }

        Ok(table)
    }

    /// Commit one generic parameter as a generic parameter.
    fn commit_generic_parameter(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        template: dir::LocalGenericTemplateId,
        generic: GenericParameterBinding,
    ) -> CompilerResult<dir::GenericParameterBinding> {
        let parameter = match generic {
            GenericParameterBinding::Type {
                parameter,
                variance,
                constraint,
                default,
            } => {
                let constraint = self.commit_generic_constraint(
                    module,
                    output,
                    environment,
                    parameter.template,
                    constraint,
                )?;
                let default = self.commit_generic_type_default(
                    module,
                    output,
                    environment,
                    parameter.template,
                    default,
                )?;

                dir::GenericParameterBinding::Type {
                    template,
                    key: parameter.key,
                    variance,
                    constraint,
                    default,
                    origin: parameter.origin,
                }
            }
            GenericParameterBinding::VariadicType {
                parameter,
                variance,
                constraint,
                default,
            } => {
                let constraint = self.commit_generic_constraint(
                    module,
                    output,
                    environment,
                    parameter.template,
                    constraint,
                )?;
                let default = self.commit_generic_type_default(
                    module,
                    output,
                    environment,
                    parameter.template,
                    default,
                )?;

                dir::GenericParameterBinding::VariadicType {
                    template,
                    key: parameter.key,
                    variance,
                    constraint,
                    default,
                    origin: parameter.origin,
                }
            }
            GenericParameterBinding::Static {
                parameter,
                ty,
                default,
            } => {
                let constraint = self.commit_generic_constraint(
                    module,
                    output,
                    environment,
                    parameter.template,
                    ty,
                )?;
                let default = if let Some(operand) = default {
                    self.commit_closed_static_operand(module, output, environment, operand)?
                } else {
                    None
                };

                dir::GenericParameterBinding::Static {
                    template,
                    key: parameter.key,
                    constraint,
                    default,
                    origin: parameter.origin,
                }
            }
            GenericParameterBinding::VariadicStatic {
                parameter,
                ty,
                default,
            } => {
                let constraint = self.commit_generic_constraint(
                    module,
                    output,
                    environment,
                    parameter.template,
                    ty,
                )?;
                let default = if let Some(operand) = default {
                    self.commit_closed_static_operand(module, output, environment, operand)?
                } else {
                    None
                };

                dir::GenericParameterBinding::VariadicStatic {
                    template,
                    key: parameter.key,
                    constraint,
                    default,
                    origin: parameter.origin,
                }
            }
        };

        Ok(parameter)
    }

    /// Commit a generic constraint from its declared type term.
    fn commit_generic_constraint(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        template: dir::GlobalGenericTemplateId,
        constraint: Option<TypeOperand>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(constraint) = constraint else {
            return Ok(None);
        };

        let source = self.generic_template_source(template)?;
        let constraint =
            self.commit_type_operand(module, output, environment, constraint, source.local_id);

        constraint
    }

    /// Commit a generic type parameter default.
    fn commit_generic_type_default(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        template: dir::GlobalGenericTemplateId,
        default: Option<TypeOperand>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(default) = default else {
            return Ok(None);
        };

        let source = self.generic_template_source(template)?;
        let default =
            self.commit_type_operand(module, output, environment, default, source.local_id)?;

        Ok(default)
    }

    /// Return the source node that declared one generic template.
    fn generic_template_source(
        &self,
        template: dir::GlobalGenericTemplateId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let Some(template) = self.inference.generic_template(template) else {
            return Err(CompilerError::Internal {
                message: format!("generic template {template:?} is not allocated"),
            });
        };

        Ok(template.source)
    }
}
