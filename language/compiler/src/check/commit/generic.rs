use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, CheckModuleState, Constraint, GenericInstance, GenericParameter, TypeTerm,
};

impl CheckComponentState<'_> {
    /// Commit generic instances from solved type reference constraints.
    pub(super) fn commit_generic_instance_table(&mut self) -> CompilerResult<()> {
        let constraints = self.collect_constraints();

        // write type expression applications after their arguments are solved
        for constraint in constraints {
            let Constraint::DefineType {
                result: _,
                term:
                    TypeTerm::Reference {
                        source: Some(source),
                        symbol,
                        arguments,
                    },
                origin: _,
            } = constraint
            else {
                continue;
            };
            if arguments.is_empty() {
                continue;
            }

            let environment = self.environment.clone();
            let instance = GenericInstance { symbol, arguments };
            let check_module = self.module_mut(source.module_id)?;

            if check_module
                .commit_generic_instance(environment.as_ref(), source, &instance)
                .is_none()
            {
                continue;
            }
        }

        Ok(())
    }
}

impl CheckModuleState {
    /// Commit generic parameters into the checked generic slot table.
    pub(super) fn commit_generic_slot_table(&mut self, environment: &GlobalEnvironment) {
        let mut generics = self
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner.module_id == self.input.module)
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
