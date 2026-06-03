use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, GenericSlot, GenericSlotHeader, GenericSlotId};

impl CheckState<'_> {
    /// Return one generic parameter slot in a component module context.
    pub(in crate::check) fn import_generic_parameter_slot(
        &mut self,
        module: ModuleId,
        parameter: dir::GenericParameterRef,
    ) -> GenericSlotId {
        let slot_id = GenericSlotId::from(parameter);

        if self.inference.generic_slot_by_id(slot_id).is_some() {
            return slot_id;
        }

        if self.is_component_module(parameter.owner.module_id) {
            panic!("generic parameter {parameter:?} has no check slot");
        }

        let dependency = parameter.owner.module_id;
        let slot = self.import_generic_parameter(parameter);
        let generic = self.import_generic_slot(module, slot, dependency);

        self.insert_generic_slot(generic);

        slot_id
    }

    /// Return one committed generic slot by parameter identity.
    fn import_generic_parameter(&self, parameter: dir::GenericParameterRef) -> dir::GenericSlot {
        let dependency = self.dependency(parameter.owner.module_id);

        // find the matching committed slot
        for (_, slot) in dependency.generics.iter_slots() {
            let template = dependency.generics.get_template(slot.template());
            if template.owner == parameter.owner
                && slot.key() == parameter.key
                && slot.index() == parameter.index
            {
                return *slot;
            }
        }

        panic!("dependency generic parameter {parameter:?} has no slot")
    }

    /// Import one generic slot.
    fn import_generic_slot(
        &mut self,
        module: ModuleId,
        slot: dir::GenericSlot,
        dependency: ModuleId,
    ) -> GenericSlot {
        let owner = self
            .dependency(dependency)
            .generics
            .get_template(slot.template())
            .owner;
        let header = GenericSlotHeader {
            owner,
            key: slot.key(),
            index: slot.index(),
            origin: slot.origin(),
        };

        match slot {
            dir::GenericSlot::Type {
                variance,
                constraint,
                default,
                ..
            } => GenericSlot::Type {
                slot: header,
                variance,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_type_operand(module, id)),
            },
            dir::GenericSlot::VariadicType {
                variance,
                constraint,
                default,
                ..
            } => GenericSlot::VariadicType {
                slot: header,
                variance,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_type_operand(module, id)),
            },
            dir::GenericSlot::Static {
                constraint,
                default,
                ..
            } => GenericSlot::Static {
                slot: header,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_static_operand(module, id)),
            },
            dir::GenericSlot::VariadicStatic {
                constraint,
                default,
                ..
            } => GenericSlot::VariadicStatic {
                slot: header,
                constraint: constraint.map(|id| self.import_type_operand(module, id)),
                default: default.map(|id| self.import_static_operand(module, id)),
            },
        }
    }
}
