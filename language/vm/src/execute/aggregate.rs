use std::ops::Range;

use destack_bytecode::{Instruction, Opcode};
use destack_mir::{DiscriminantField, VariantEncoding};
use destack_program::{Layout, LayoutId, LayoutShape, VariantLayout};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one packed aggregate or variant operation.
    pub(crate) fn execute_aggregate(&mut self, instruction: Instruction<'_>) -> Result<()> {
        match instruction.opcode() {
            Opcode::AGGREGATE => self.execute_aggregate_new(instruction),
            Opcode::EXTRACT => self.execute_extract(instruction),
            Opcode::INSERT => self.execute_insert(instruction),
            Opcode::VARIANT_NEW => self.execute_variant_new(instruction),
            Opcode::VARIANT_TAG => self.execute_variant_tag(instruction),
            _ => unreachable!("aggregate dispatch selects one aggregate opcode"),
        }
    }

    /// Construct one packed aggregate from physical source placements.
    fn execute_aggregate_new(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.span()?;
        let placements = operands.placements()?;
        let result = self.register_byte_range(result)?;

        // initialize padding before placing the exact source bytes
        self.machine.stack.zero(result.start, result.len())?;
        for placement in placements {
            let source = self.register_byte_range(placement.registers)?;
            let byte_offset = placement.byte_offset as usize;
            let byte_len = placement.byte_len as usize;
            self.copy_register_bytes(source, &result, byte_offset, byte_len)?;
        }

        Ok(())
    }

    /// Extract one physical byte range from a packed value.
    fn execute_extract(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.span()?;
        let source = operands.span()?;
        let byte_offset = operands.u32()? as usize;
        let byte_len = operands.u32()? as usize;
        let result = self.register_byte_range(result)?;
        let source = self.register_byte_range(source)?;
        if byte_len > result.len() || byte_offset + byte_len > source.len() {
            return Err(self.invalid_instruction());
        }

        // clear register padding before copying the selected bytes
        self.machine.stack.zero(result.start, result.len())?;
        self.machine
            .stack
            .move_bytes(source.start + byte_offset, result.start, byte_len);

        Ok(())
    }

    /// Replace one physical byte range inside a packed value.
    fn execute_insert(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.span()?;
        let source = operands.span()?;
        let byte_offset = operands.u32()? as usize;
        let byte_len = operands.u32()? as usize;
        let value = operands.span()?;
        let result = self.register_byte_range(result)?;
        let source = self.register_byte_range(source)?;
        let value = self.register_byte_range(value)?;

        // retain the complete original value before replacing the selected bytes
        if result.len() != source.len() {
            return Err(self.invalid_instruction());
        }
        self.machine
            .stack
            .move_bytes(source.start, result.start, source.len());

        // write the replacement from the start of its register value
        self.copy_register_bytes(value, &result, byte_offset, byte_len)
    }

    /// Construct one variant from its linked physical layout.
    fn execute_variant_new(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.span()?;
        let layout = operands.u32()?;
        let layout = LayoutId::from_raw(layout).ok_or_else(|| self.invalid_instruction())?;
        let case_index = operands.u32()?;
        let payload = operands.span()?;
        let layout = self.layout(layout)?;
        let variant = self.variant(&layout)?;
        let case = *self
            .machine
            .program
            .variant_cases(variant)
            .get(case_index as usize)
            .ok_or_else(|| self.invalid_instruction())?;
        let result = self.register_byte_range(result)?;

        // initialize padding and place the optional payload
        self.check_layout_range(&layout, &result)?;
        self.machine.stack.zero(result.start, result.len())?;
        if payload.word_count > 0 {
            let payload = self.register_byte_range(payload)?;
            let payload_layout = self
                .machine
                .program
                .layout(case.ty)
                .ok_or_else(|| self.invalid_instruction())?;
            self.copy_register_bytes(
                payload,
                &result,
                case.payload_offset as usize,
                payload_layout.byte_len(),
            )?;
        }

        // encode the selected case into its discriminant field
        let field = variant.encoding.field();
        let scalar = self.read_discriminant(result.start, field)?;
        let scalar = match variant.encoding {
            VariantEncoding::Direct { field } => field.insert(scalar, case.discriminant.bits()),
            VariantEncoding::Niche { .. } => variant
                .encoding
                .encode_niche(scalar, case_index)
                .ok_or_else(|| self.invalid_instruction())?,
        };

        self.write_discriminant(result.start, field, scalar)
    }

    /// Read one variant's logical discriminant from its linked physical layout.
    fn execute_variant_tag(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let result = operands.span()?;
        let source = operands.span()?;
        let layout = operands.u32()?;
        let layout = LayoutId::from_raw(layout).ok_or_else(|| self.invalid_instruction())?;
        let layout = self.layout(layout)?;
        let variant = self.variant(&layout)?;
        let source = self.register_byte_range(source)?;

        // decode the physical discriminant into its logical value
        self.check_layout_range(&layout, &source)?;
        let field = variant.encoding.field();
        let scalar = self.read_discriminant(source.start, field)?;
        let discriminant = match variant.encoding {
            VariantEncoding::Direct { field } => field.extract(scalar),
            VariantEncoding::Niche { .. } => {
                let case = variant
                    .encoding
                    .decode_niche(scalar)
                    .and_then(|case| {
                        self.machine
                            .program
                            .variant_cases(variant)
                            .get(case as usize)
                    })
                    .ok_or_else(|| self.invalid_instruction())?;

                case.discriminant.bits()
            }
        };

        // write the discriminant into its exact result width
        let result = self.register_byte_range(result)?;
        let bytes = discriminant.to_le_bytes();
        if result.len() > bytes.len() {
            return Err(self.invalid_instruction());
        }
        self.machine
            .stack
            .write_bytes(result.start, &bytes[..result.len()])?;

        Ok(())
    }

    /// Return one copied Program layout.
    fn layout(&self, id: LayoutId) -> Result<Layout> {
        self.machine
            .program
            .layout_by_id(id)
            .copied()
            .ok_or_else(|| self.invalid_instruction())
    }

    /// Return one variant layout.
    fn variant(&self, layout: &Layout) -> Result<VariantLayout> {
        match layout.shape {
            LayoutShape::Variant(variant) => Ok(variant),
            _ => Err(self.invalid_instruction()),
        }
    }

    /// Check that one register range contains one complete layout.
    fn check_layout_range(&self, layout: &Layout, range: &Range<usize>) -> Result<()> {
        if layout.byte_len() > range.len() {
            return Err(self.invalid_instruction());
        }

        Ok(())
    }

    /// Copy one exact source range into a destination register value.
    fn copy_register_bytes(
        &mut self,
        source: Range<usize>,
        target: &Range<usize>,
        byte_offset: usize,
        byte_len: usize,
    ) -> Result<()> {
        if byte_len > source.len() || byte_offset + byte_len > target.len() {
            return Err(self.invalid_instruction());
        }
        self.machine
            .stack
            .move_bytes(source.start, target.start + byte_offset, byte_len);

        Ok(())
    }

    /// Read one physical variant discriminant field.
    fn read_discriminant(&self, base: usize, field: DiscriminantField) -> Result<u128> {
        let byte_len = field.byte_len as usize;
        if byte_len > size_of::<u128>() {
            return Err(self.invalid_instruction());
        }
        let mut bytes = [0; size_of::<u128>()];
        self.machine
            .stack
            .read_bytes(base + field.offset as usize, &mut bytes[..byte_len])?;

        Ok(u128::from_le_bytes(bytes))
    }

    /// Write one physical variant discriminant field.
    fn write_discriminant(
        &mut self,
        base: usize,
        field: DiscriminantField,
        scalar: u128,
    ) -> Result<()> {
        let byte_len = field.byte_len as usize;
        if byte_len > size_of::<u128>() {
            return Err(self.invalid_instruction());
        }
        let bytes = scalar.to_le_bytes();
        self.machine
            .stack
            .write_bytes(base + field.offset as usize, &bytes[..byte_len])?;

        Ok(())
    }
}
