use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::CompilerResult;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Definition, GenericInstance, GenericParameter, TypeTerm, VariableId,
};

impl CheckState<'_> {
    /// Commit generic instances from solved type reference definitions.
    pub(super) fn commit_generic_instance_table(&mut self) -> CompilerResult<()> {
        let definitions = self.collect_definitions();

        // write type expression applications after their arguments are solved
        for definition in definitions {
            let Definition::Type {
                result: _,
                term,
                origin: _,
                condition: _,
            } = definition
            else {
                continue;
            };
            let TypeTerm::Reference {
                source: Some(source),
                symbol,
                arguments,
            } = self.terms.get(term).clone()
            else {
                continue;
            };
            if arguments.is_empty() {
                continue;
            }

            let environment = self.environment.clone();
            let instance = GenericInstance { symbol, arguments };

            if self
                .commit_generic_instance(environment.as_ref(), source, &instance)
                .is_none()
            {
                continue;
            }
        }

        Ok(())
    }
}

impl CheckState<'_> {
    /// Commit generic parameters into the checked generic slot table.
    pub(super) fn commit_generic_slot_table(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
    ) {
        let mut generics = self
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner.module_id == module)
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

            self.output_mut(module).generics.push_slot(slot);
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
                    .and_then(|variable| self.commit_generic_constraint(environment, variable)),
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
                    .and_then(|variable| self.commit_generic_constraint(environment, variable)),
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
                    .and_then(|variable| self.commit_generic_constraint(environment, variable)),
                default: default
                    .and_then(|variable| self.commit_variable_static(environment, variable)),
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
                    .and_then(|variable| self.commit_generic_constraint(environment, variable)),
                default: default
                    .and_then(|variable| self.commit_variable_static(environment, variable)),
                origin: slot.origin,
            },
        }
    }

    /// Commit a generic constraint from its declared type term.
    fn commit_generic_constraint(
        &mut self,
        environment: &GlobalEnvironment,
        variable: VariableId,
    ) -> Option<dir::LocalTypeId> {
        self.commit_variable_type_definition(environment, variable)
            .or_else(|| self.commit_variable_type(environment, variable))
    }
}
