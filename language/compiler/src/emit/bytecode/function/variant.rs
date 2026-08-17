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
        variant: mir::Value,
    ) -> Result<(), EmitError> {
        let variant_type = self.optimized.tree.storage_type(self.value_type(variant)?);
        let variant_type = self.types.pointee(variant_type)?;
        let opcode = bytecode::Opcode::variant_tag_load(self.address(variant)?);
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(variant)?);
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
        let variant_type = self.optimized.tree.storage_type(self.value_type(variant)?);
        let (byte_offset, byte_len) = self.types.variant(variant_type, case)?;

        self.emit_extract(destination, variant, byte_offset, byte_len)
    }

    /// Get one stored variant payload's address.
    pub(super) fn emit_variant_payload_address(
        &mut self,
        destination: mir::Value,
        variant: mir::Value,
        case: u32,
    ) -> Result<(), EmitError> {
        let variant_type = self.value_type(variant)?;
        let variant_type = self.types.pointee(variant_type)?;
        let (byte_offset, _) = self.types.variant(variant_type, case)?;

        self.emit_address_add_immediate(destination, variant, byte_offset)
    }
}
