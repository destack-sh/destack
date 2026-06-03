use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, GenericParameterBinding, GenericParameterId, GenericParameterIdentity,
};

impl CheckState<'_> {
    /// Import one committed generic template.
    pub(in crate::check) fn import_generic_template(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
    ) {
        if self.inference.generic_template_for_owner(owner).is_some() {
            return;
        }

        if self.is_component_module(owner.module_id) {
            return;
        }

        let dependency = self.dependency(owner.module_id);
        let Some((_, template)) = dependency
            .generics
            .iter_templates()
            .find(|(_, template)| template.owner == owner)
        else {
            return;
        };
        let parameters = template
            .parameters
            .iter()
            .map(|parameter_id| {
                (
                    (*parameter_id).into_global(owner.module_id),
                    *dependency.generics.get_parameter(*parameter_id),
                )
            })
            .collect::<Vec<_>>();

        for (parameter_id, parameter) in parameters {
            let generic = self.import_generic_parameter(module, owner, parameter_id, parameter);

            self.insert_generic_parameter(generic);
        }
    }

    /// Return one generic parameter parameter in a component module context.
    pub(in crate::check) fn import_generic_parameter_id(
        &mut self,
        module: ModuleId,
        parameter: dir::GlobalGenericParameterId,
    ) -> GenericParameterId {
        if self.is_component_module(parameter.module_id) {
            return parameter;
        }

        if self.inference.generic_parameter_by_id(parameter).is_some() {
            return parameter;
        }

        let dependency = self.dependency(parameter.module_id);
        let generic = dependency.generics.get_parameter(parameter.local_id);
        let template = dependency.generics.get_template(generic.template());
        let owner = template.owner;

        self.import_generic_template(module, owner);
        if self.inference.generic_parameter_by_id(parameter).is_none() {
            panic!("dependency generic parameter {parameter:?} has no parameter");
        }

        parameter
    }

    /// Import one generic parameter.
    fn import_generic_parameter(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        id: dir::GlobalGenericParameterId,
        parameter: dir::GenericParameterBinding,
    ) -> GenericParameterBinding {
        let header = GenericParameterIdentity {
            id,
            owner,
            key: parameter.key(),
            origin: parameter.origin(),
        };

        match parameter {
            dir::GenericParameterBinding::Type {
                variance,
                constraint,
                default,
                ..
            } => GenericParameterBinding::Type {
                identity: header,
                variance,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_type_operand(module, id)),
            },
            dir::GenericParameterBinding::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => GenericParameterBinding::VariadicType {
                identity: header,
                variance,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_type_operand(module, id)),
            },
            dir::GenericParameterBinding::Static {
                constraint,
                default,
                ..
            } => GenericParameterBinding::Static {
                identity: header,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_static_operand(module, id)),
            },
            dir::GenericParameterBinding::VariadicStatic {
                constraint,
                default,
                ..
            } => GenericParameterBinding::VariadicStatic {
                identity: header,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_static_operand(module, id)),
            },
        }
    }
}
