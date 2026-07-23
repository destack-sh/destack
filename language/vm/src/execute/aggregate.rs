use std::ops::Range;

use destack_bytecode::{Instruction, Opcode, RegisterId, RegisterRange};
use destack_mir::{DiscriminantField, VariantEncoding};
use destack_program::{ElementLayout, Layout, LayoutShape, TypeId, VariantLayout, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one packed aggregate or variant operation.
    pub(crate) fn execute_aggregate(&mut self, instruction: Instruction<'_>) -> Result<()> {
        match instruction.opcode() {
            Opcode::AGGREGATE => self.execute_aggregate_new(instruction),
            Opcode::FIELD_GET => self.execute_field_get(instruction),
            Opcode::FIELD_SET => self.execute_field_set(instruction),
            Opcode::ELEMENT_GET => self.execute_element_get(instruction),
            Opcode::ELEMENT_SET => self.execute_element_set(instruction),
            Opcode::VARIANT_NEW => self.execute_variant_new(instruction),
            Opcode::VARIANT_TAG => self.execute_variant_tag(instruction),
            Opcode::VARIANT_PAYLOAD => self.execute_variant_payload(instruction),
            _ => unreachable!("aggregate dispatch selects one aggregate opcode"),
        }
    }

    /// Execute one aggregate construction from logical source values.
    fn execute_aggregate_new(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let mut values = operands
            .registers()
            .map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let field_count = self.machine.program.layout_field_count(&layout);
        if field_count != Some(values.len()) {
            return Err(self.invalid_instruction());
        }

        // initialize every padding byte before writing logical fields
        let result = self.register_byte_range(result)?;
        self.check_layout_range(&layout, &result)?;
        self.machine.stack.zero(result.start, result.len())?;

        // place each source value at its physical field offset
        for (index, value) in values.by_ref().enumerate() {
            let field = *self
                .machine
                .program
                .layout_field(&layout, index as u32)
                .ok_or_else(|| self.invalid_instruction())?;
            let source = self.register_bytes(value, field.size as usize)?;
            self.copy_value(
                source,
                result.start + field.offset as usize,
                field.size as usize,
            )?;
        }

        Ok(())
    }

    /// Execute one aggregate field projection.
    fn execute_field_get(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let index = operands.u32().map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let field = *self
            .machine
            .program
            .layout_field(&layout, index)
            .ok_or_else(|| self.invalid_instruction())?;

        // copy the exact field bytes into one initialized register value
        let source = self.register_byte_range(source)?;
        self.check_layout_range(&layout, &source)?;
        let result = self.register_byte_range(result)?;
        self.machine.stack.zero(result.start, result.len())?;
        self.copy_value(
            source.start + field.offset as usize
                ..source.start + field.offset as usize + field.size as usize,
            result.start,
            field.size as usize,
        )
    }

    /// Execute one persistent aggregate field update.
    fn execute_field_set(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let index = operands.u32().map_err(|_| self.invalid_instruction())?;
        let value = operands.range().map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let field = *self
            .machine
            .program
            .layout_field(&layout, index)
            .ok_or_else(|| self.invalid_instruction())?;

        // retain the original aggregate before replacing one field
        let result = self.register_byte_range(result)?;
        let source = self.register_byte_range(source)?;
        self.check_layout_range(&layout, &result)?;
        self.check_layout_range(&layout, &source)?;
        self.machine
            .stack
            .move_bytes(source.start, result.start, layout.byte_len());

        // overwrite the selected field from its complete logical value
        let value = self.register_byte_range(value)?;
        self.copy_value(
            value,
            result.start + field.offset as usize,
            field.size as usize,
        )
    }

    /// Execute one fixed-array element projection.
    fn execute_element_get(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let index = operands.u32().map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let element = self.element(&layout, index)?;
        let element_layout = self.layout(element.element)?;

        // copy one statically selected element into initialized registers
        let source = self.register_byte_range(source)?;
        self.check_layout_range(&layout, &source)?;
        let result = self.register_byte_range(result)?;
        self.machine.stack.zero(result.start, result.len())?;
        let source_start = source.start + index as usize * element.stride as usize;
        self.copy_value(
            source_start..source_start + element_layout.byte_len(),
            result.start,
            element_layout.byte_len(),
        )
    }

    /// Execute one persistent fixed-array element update.
    fn execute_element_set(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let index = operands.u32().map_err(|_| self.invalid_instruction())?;
        let value = operands.range().map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let element = self.element(&layout, index)?;
        let element_layout = self.layout(element.element)?;

        // retain the original array before replacing one element
        let result = self.register_byte_range(result)?;
        let source = self.register_byte_range(source)?;
        self.check_layout_range(&layout, &result)?;
        self.check_layout_range(&layout, &source)?;
        self.machine
            .stack
            .move_bytes(source.start, result.start, layout.byte_len());

        // overwrite the statically selected element
        let value = self.register_byte_range(value)?;
        let target = result.start + index as usize * element.stride as usize;
        self.copy_value(value, target, element_layout.byte_len())
    }

    /// Execute one variant construction.
    fn execute_variant_new(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let case_index = operands.u32().map_err(|_| self.invalid_instruction())?;
        let payload = operands.range().map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let variant = self.variant(&layout)?;
        let case = *self
            .machine
            .program
            .variant_cases(variant)
            .get(case_index as usize)
            .ok_or_else(|| self.invalid_instruction())?;
        let result = self.register_byte_range(result)?;
        self.check_layout_range(&layout, &result)?;
        self.machine.stack.zero(result.start, result.len())?;

        // place the optional payload before encoding a possible payload niche
        if payload.word_count > 0 {
            let payload_layout = self.layout(case.ty)?;
            let payload = self.register_byte_range(payload)?;
            self.copy_value(
                payload,
                result.start + case.payload_offset as usize,
                payload_layout.byte_len(),
            )?;
        }

        // encode the selected case into its physical discriminant field
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

    /// Execute one variant discriminant projection.
    fn execute_variant_tag(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let layout = self.layout(ty)?;
        let variant = self.variant(&layout)?;
        let source = self.register_byte_range(source)?;
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

        // write the logical discriminant into its declared register width
        let result = self.register_byte_range(result)?;
        let bytes = discriminant.to_le_bytes();
        if result.len() > bytes.len() {
            return Err(self.invalid_instruction());
        }
        self.machine
            .stack
            .write_bytes(result.start, &bytes[..result.len()]);

        Ok(())
    }

    /// Execute one statically selected variant payload projection.
    fn execute_variant_payload(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let result = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let case_index = operands.u32().map_err(|_| self.invalid_instruction())?;
        let layout = self.layout(ty)?;
        let variant = self.variant(&layout)?;
        let case = *self
            .machine
            .program
            .variant_cases(variant)
            .get(case_index as usize)
            .ok_or_else(|| self.invalid_instruction())?;
        let payload_layout = self.layout(case.ty)?;

        // copy the statically selected payload into initialized registers
        let source = self.register_byte_range(source)?;
        self.check_layout_range(&layout, &source)?;
        let result = self.register_byte_range(result)?;
        self.machine.stack.zero(result.start, result.len())?;
        let source_start = source.start + case.payload_offset as usize;
        self.copy_value(
            source_start..source_start + payload_layout.byte_len(),
            result.start,
            payload_layout.byte_len(),
        )
    }

    /// Return one copied Program layout.
    fn layout(&self, ty: TypeId) -> Result<Layout> {
        self.machine
            .program
            .layout(ty)
            .copied()
            .ok_or_else(|| self.invalid_instruction())
    }

    /// Return one fixed element layout and require an in-bounds index.
    fn element(&self, layout: &Layout, index: u32) -> Result<ElementLayout> {
        let element = match layout.shape {
            LayoutShape::Array(element) | LayoutShape::Vector(element) => element,
            _ => return Err(self.invalid_instruction()),
        };
        if index >= element.count {
            return Err(self.invalid_instruction());
        }

        Ok(element)
    }

    /// Return one variant layout.
    fn variant(&self, layout: &Layout) -> Result<VariantLayout> {
        match layout.shape {
            LayoutShape::Variant(variant) => Ok(variant),
            _ => Err(self.invalid_instruction()),
        }
    }

    /// Return an exact byte range beginning at one register.
    fn register_bytes(&self, register: RegisterId, byte_len: usize) -> Result<Range<usize>> {
        let frame = self.frame();
        let start = frame.range(RegisterRange::new(register, 0)) * Word::BYTE_LEN;
        let register_end = (frame.register_offset + frame.register_count as usize) * Word::BYTE_LEN;
        let end = start + byte_len;
        if end > register_end {
            return Err(self.invalid_instruction());
        }

        Ok(start..end)
    }

    /// Check that one register range contains one complete layout.
    fn check_layout_range(&self, layout: &Layout, range: &Range<usize>) -> Result<()> {
        if layout.byte_len() > range.len() {
            return Err(self.invalid_instruction());
        }

        Ok(())
    }

    /// Copy one exact logical value into packed destination bytes.
    fn copy_value(&mut self, source: Range<usize>, target: usize, byte_len: usize) -> Result<()> {
        if byte_len > source.len() {
            return Err(self.invalid_instruction());
        }
        self.machine
            .stack
            .move_bytes(source.start, target, byte_len);

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
            .read_bytes(base + field.offset as usize, &mut bytes[..byte_len]);

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
            .write_bytes(base + field.offset as usize, &bytes[..byte_len]);

        Ok(())
    }
}
