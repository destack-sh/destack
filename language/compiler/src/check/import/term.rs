use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, StaticOperand, StaticTerm, TypeOperand, TypeTerm};

impl CheckState<'_> {
    /// Return one committed type id as a check type operand.
    pub(in crate::check) fn import_type_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalTypeId,
    ) -> TypeOperand {
        if let Some(operand) = self.inputs.type_id_operand(source) {
            return operand;
        }

        // import generic parameters as check generic slots
        let parameter = match self.r#type(source) {
            dir::Type::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            let slot = self.import_generic_parameter_slot(module, parameter);
            let operand = TypeOperand::Term(self.inference.push_term(TypeTerm::Parameter(slot)));

            self.inputs.insert_type_id_operand(source, operand);

            return operand;
        }

        TypeOperand::Type(source)
    }

    /// Return one committed static id as a check static operand.
    pub(in crate::check) fn import_static_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalStaticId,
    ) -> StaticOperand {
        if let Some(operand) = self.inputs.static_id_operand(source) {
            return operand;
        }

        // import generic parameters as check generic slots
        let parameter = match self.r#static(source) {
            dir::StaticTerm::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            let slot = self.import_generic_parameter_slot(module, parameter);
            let operand =
                StaticOperand::Term(self.inference.push_term(StaticTerm::Parameter(slot)));

            self.inputs.insert_static_id_operand(source, operand);

            return operand;
        }

        StaticOperand::Static(source)
    }
}
