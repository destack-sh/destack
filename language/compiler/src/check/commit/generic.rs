use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::check::{CheckModuleState, GenericParameter};

impl CheckModuleState {
    /// Commit generic parameters into the checked generic table.
    pub(super) fn commit_generics(&mut self, environment: &GlobalEnvironment) {
        let mut generics = self
            .generic_parameters()
            .map(|(variable, generic)| {
                (
                    generic.slot().owner,
                    generic.slot().index,
                    variable,
                    generic.clone(),
                )
            })
            .collect::<Vec<_>>();
        generics.sort_by_key(|(owner, index, _, _)| (*owner, *index));

        // write every generic parameter as a real generic slot
        for (_, _, _, generic) in generics {
            let slot = self.commit_generic_slot(environment, generic);

            self.output.generics.push_slot(slot);
        }
    }

    /// Commit one generic parameter as a generic slot.
    fn commit_generic_slot(
        &mut self,
        environment: &GlobalEnvironment,
        generic: GenericParameter,
    ) -> dir::GenericSlot {
        match generic {
            GenericParameter::Type {
                slot,
                variance,
                constraint,
                default,
            } => dir::GenericSlot::Type {
                owner: slot.owner,
                key: slot.key,
                index: slot.index,
                variance,
                constraint: constraint
                    .and_then(|variable| self.commit_variable_type(environment, variable)),
                default: default
                    .and_then(|variable| self.commit_variable_type(environment, variable)),
                origin: slot.origin,
            },
            GenericParameter::VariadicType {
                slot,
                variance,
                constraint,
                default,
            } => dir::GenericSlot::VariadicType {
                owner: slot.owner,
                key: slot.key,
                index: slot.index,
                variance,
                constraint: constraint
                    .and_then(|variable| self.commit_variable_type(environment, variable)),
                default: default
                    .and_then(|variable| self.commit_variable_type(environment, variable)),
                origin: slot.origin,
            },
            GenericParameter::Static {
                slot,
                constraint,
                default,
            } => dir::GenericSlot::Static {
                owner: slot.owner,
                key: slot.key,
                index: slot.index,
                constraint: constraint
                    .and_then(|variable| self.commit_variable_type(environment, variable)),
                default: default.and_then(|variable| self.commit_variable_static(variable)),
                origin: slot.origin,
            },
            GenericParameter::VariadicStatic {
                slot,
                constraint,
                default,
            } => dir::GenericSlot::VariadicStatic {
                owner: slot.owner,
                key: slot.key,
                index: slot.index,
                constraint: constraint
                    .and_then(|variable| self.commit_variable_type(environment, variable)),
                default: default.and_then(|variable| self.commit_variable_static(variable)),
                origin: slot.origin,
            },
        }
    }
}
