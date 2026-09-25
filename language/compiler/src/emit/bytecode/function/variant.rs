use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Construct one packed variant case.
    pub(super) fn emit_variant_new(
        &mut self,
        destination: mir::Value,
        case: u32,
        payload: Option<mir::Value>,
        result_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        // encode the variant tag and payload
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::VARIANT_NEW);
        let result_type = self.optimized.tree.storage_type(result_type);
        let result_type = self.types.type_id(result_type)?;
        instruction.relocation(bytecode::RelocationTag::LAYOUT, result_type.0);
        instruction.u32(case);
        let payload = payload
            .map(|payload| self.register(payload))
            .transpose()?
            .unwrap_or_else(bytecode::RegisterSpan::empty);
        instruction.span(payload);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Read one packed variant's logical discriminant.
    pub(super) fn emit_variant_tag(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
    ) -> Result<(), EmitError> {
        // read the variant storage type
        let ty = self
            .function
            .value_type(variant)
            .ok_or_else(|| self.internal("missing variant type"))?;
        let ty = self.optimized.tree.storage_type(ty);
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::VARIANT_TAG);
        instruction.span(self.register(variant)?);
        let ty = self.types.type_id(ty)?;
        instruction.relocation(bytecode::RelocationTag::LAYOUT, ty.0);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Read one stored variant's logical discriminant.
    pub(super) fn emit_variant_tag_load(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
    ) -> Result<(), EmitError> {
        // read the selected variant layout
        let Some(mir::PlaceType::Value(variant_type)) =
            place.ty(self.function_id, &self.optimized.tree)
        else {
            return Err(self.internal("variant tag load does not select a value"));
        };
        let variant_type = self.optimized.tree.storage_type(variant_type);
        let selected = self.emit_place(place, None)?;
        let opcode = bytecode::Opcode::variant_tag_load(selected.kind);
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(selected.address);
        let variant_type = self.types.type_id(variant_type)?;
        instruction.relocation(bytecode::RelocationTag::LAYOUT, variant_type.0);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Read one statically selected variant payload.
    pub(super) fn emit_variant_payload(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
        case: u32,
    ) -> Result<(), EmitError> {
        // read the variant payload layout
        let variant_type = self.optimized.tree.storage_type(self.value_type(variant)?);
        let (byte_offset, byte_len) = self.types.variant(variant_type, case)?;

        self.emit_extract(destination, variant, byte_offset, byte_len)
    }
}
