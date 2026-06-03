use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use destack_source::ModuleId;

use crate::check::{CheckState, GenericSlot, TypeOperand};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit generic parameters into the checked generic slot table.
    pub(super) fn commit_generic_slot_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> dir::GenericSegment {
        let mut table = dir::GenericSegment::new(module);
        let mut generics = self
            .inference
            .generic_slots()
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

        // write each owner as one generic template
        let mut index = 0;
        while index < generics.len() {
            let owner = generics[index].0;
            let template_id = dir::LocalGenericTemplateId::new(table.template_count());
            let mut slots = Vec::new();

            while index < generics.len() && generics[index].0 == owner {
                let generic = generics[index].3.clone();
                let slot =
                    self.commit_generic_slot(module, output, environment, template_id, generic);
                let slot_id = table.push_slot(slot);

                slots.push(slot_id);
                index += 1;
            }

            table.push_template(dir::GenericTemplate { owner, slots });
        }

        table
    }

    /// Commit one generic parameter as a generic slot.
    fn commit_generic_slot(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        template: dir::LocalGenericTemplateId,
        generic: GenericSlot,
    ) -> dir::GenericSlot {
        match generic {
            GenericSlot::Type {
                slot,
                variance,
                constraint,
                default,
            } => dir::GenericSlot::Type {
                template,
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
                default: default.and_then(|operand| {
                    let source = self
                        .module(slot.owner.module_id)
                        .symbol_declaration_node(slot.owner.local_id);

                    self.commit_type_operand(module, output, environment, operand, source)
                }),
                origin: slot.origin,
            },
            GenericSlot::VariadicType {
                slot,
                variance,
                constraint,
                default,
            } => dir::GenericSlot::VariadicType {
                template,
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
                default: default.and_then(|operand| {
                    let source = self
                        .module(slot.owner.module_id)
                        .symbol_declaration_node(slot.owner.local_id);

                    self.commit_type_operand(module, output, environment, operand, source)
                }),
                origin: slot.origin,
            },
            GenericSlot::Static {
                slot,
                constraint,
                default,
            } => dir::GenericSlot::Static {
                template,
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
                default: default.and_then(|operand| {
                    self.commit_static_operand(module, output, environment, operand)
                }),
                origin: slot.origin,
            },
            GenericSlot::VariadicStatic {
                slot,
                constraint,
                default,
            } => dir::GenericSlot::VariadicStatic {
                template,
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
                default: default.and_then(|operand| {
                    self.commit_static_operand(module, output, environment, operand)
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
