use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{CheckState, GenericParameterBinding, TypeOperand};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit generic parameters into the checked generic parameter table.
    pub(super) fn commit_generic_parameter_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> dir::GenericSegment {
        let mut table = dir::GenericSegment::new(module);

        // collect template parameter ids
        let mut templates =
            IndexMap::<dir::GlobalSymbolId, Vec<dir::LocalGenericParameterId>>::new();
        for (parameter_id, generic) in self.inference.generic_parameters() {
            let identity = generic.identity();
            if identity.owner.module_id != module {
                continue;
            }

            templates
                .entry(identity.owner)
                .or_default()
                .push(parameter_id.local_id);
        }

        // commit owner templates
        let mut template_ids = IndexMap::new();
        for (owner, parameters) in &templates {
            let template = dir::GenericTemplate {
                owner: *owner,
                parameters: parameters.clone(),
            };
            let template_id = table.push_template(template);

            template_ids.insert(*owner, template_id);
        }

        // commit parameters in allocated id order
        let mut parameters = self
            .inference
            .generic_parameters()
            .filter(|(_, generic)| generic.identity().owner.module_id == module)
            .map(|(parameter_id, generic)| (parameter_id, generic.clone()))
            .collect::<Vec<_>>();
        parameters.sort_by_key(|(parameter_id, _)| parameter_id.local_id);

        for (parameter_id, generic) in parameters {
            let owner = generic.identity().owner;
            let template = template_ids[&owner];
            let parameter =
                self.commit_generic_parameter(module, output, environment, template, generic);
            let committed_id = table.push_parameter(parameter);

            assert_eq!(
                committed_id, parameter_id.local_id,
                "generic parameter committed under a different id"
            );
        }

        table
    }

    /// Commit one generic parameter as a generic parameter.
    fn commit_generic_parameter(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        template: dir::LocalGenericTemplateId,
        generic: GenericParameterBinding,
    ) -> dir::GenericParameterBinding {
        match generic {
            GenericParameterBinding::Type {
                identity,
                variance,
                constraint,
                default,
            } => dir::GenericParameterBinding::Type {
                template,
                key: identity.key,
                variance,
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        identity.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|operand| {
                    let source = self
                        .module(identity.owner.module_id)
                        .symbol_declaration_node(identity.owner.local_id);

                    self.commit_type_operand(module, output, environment, operand, source)
                }),
                origin: identity.origin,
            },
            GenericParameterBinding::VariadicType {
                identity,
                variance,
                constraint,
                default,
            } => dir::GenericParameterBinding::VariadicType {
                template,
                key: identity.key,
                variance,
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        identity.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|operand| {
                    let source = self
                        .module(identity.owner.module_id)
                        .symbol_declaration_node(identity.owner.local_id);

                    self.commit_type_operand(module, output, environment, operand, source)
                }),
                origin: identity.origin,
            },
            GenericParameterBinding::Static {
                identity,
                constraint,
                default,
            } => dir::GenericParameterBinding::Static {
                template,
                key: identity.key,
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        identity.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|operand| {
                    self.commit_static_operand(module, output, environment, operand)
                }),
                origin: identity.origin,
            },
            GenericParameterBinding::VariadicStatic {
                identity,
                constraint,
                default,
            } => dir::GenericParameterBinding::VariadicStatic {
                template,
                key: identity.key,
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        identity.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|operand| {
                    self.commit_static_operand(module, output, environment, operand)
                }),
                origin: identity.origin,
            },
        }
    }

    /// Commit a generic constraint from its declared type term.
    fn commit_generic_constraint(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        owner: dir::GlobalSymbolId,
        constraint: TypeOperand,
    ) -> Option<dir::GlobalTypeId> {
        match constraint {
            TypeOperand::Variable(variable) => {
                self.commit_declared_type_variable(module, output, environment, variable)
            }
            TypeOperand::Term(term) => {
                let term = self.inference.term(term).clone();
                let source = self
                    .module(owner.module_id)
                    .symbol_declaration_node(owner.local_id);

                self.commit_type_term(owner.module_id, output, environment, &term, source)
            }
            TypeOperand::Type(ty) => Some(ty),
        }
    }
}
