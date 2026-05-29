use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::CompilerResult;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Definition, GenericInstance, GenericParameter, Origin, TypeOperand, TypeTerm,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit generic instances from solved type reference definitions.
    pub(super) fn commit_generic_instance_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        let definitions = self
            .variables
            .definitions
            .iter()
            .filter_map(|definition| {
                let Definition::Type { term, .. } = definition else {
                    return None;
                };

                Some(*term)
            })
            .collect::<Vec<_>>();

        // write type expression applications after their arguments are solved
        for term in definitions {
            let TypeTerm::Reference {
                origin: Origin::Node(source),
                symbol,
                arguments,
            } = self.terms.get(term).clone()
            else {
                continue;
            };
            if arguments.is_empty() {
                continue;
            }
            if source.module_id != module {
                continue;
            }
            let instance = GenericInstance { symbol, arguments };

            if self
                .commit_generic_instance(module, output, environment, source, &instance)
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
        output: &mut CheckModuleOutput,
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
            let slot = self.commit_generic_slot(module, output, environment, generic);

            output.generics.push_slot(slot);
        }
    }

    /// Commit one generic parameter as a generic slot.
    fn commit_generic_slot(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
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
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        slot.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|variable| {
                    self.commit_variable_type(module, output, environment, variable)
                }),
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
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        slot.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|variable| {
                    self.commit_variable_type(module, output, environment, variable)
                }),
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
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        slot.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|variable| {
                    self.commit_variable_static(module, output, environment, variable)
                }),
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
                constraint: constraint.and_then(|constraint| {
                    self.commit_generic_constraint(
                        module,
                        output,
                        environment,
                        slot.owner,
                        constraint,
                    )
                }),
                default: default.and_then(|variable| {
                    self.commit_variable_static(module, output, environment, variable)
                }),
                origin: slot.origin,
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
    ) -> Option<dir::LocalTypeId> {
        match constraint {
            TypeOperand::Variable(variable) => self
                .commit_declared_type_variable(module, output, environment, variable)
                .or_else(|| self.commit_variable_type(module, output, environment, variable)),
            TypeOperand::Term(term) => {
                let term = self.terms.get(term).clone();
                let source = self.symbol_source_node(owner);

                self.commit_type_term(owner.module_id, output, environment, &term, source)
            }
        }
    }
}
