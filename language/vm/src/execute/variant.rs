use destack_mir::VariantEncoding;
use destack_program::vm::Instruction;
use destack_program::{LayoutId, LayoutShape, TypeId, VariantCaseLayout, VariantLayout};

use crate::diagnostic::Error;
use crate::machine::Activation;

/// Construct one physically encoded variant value.
pub(crate) fn execute_variant_construct(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (byte_len, variant) = activation.variant_layout(instruction.d)?;
    let cases = activation.program.variant_cases(variant);
    let discriminant = activation.read_frame_scalar(
        instruction.b,
        activation.type_byte_len(variant.discriminant)?,
    )?;
    let case_index = cases
        .iter()
        .position(|case| case.discriminant.bits() == discriminant)
        .ok_or_else(Error::invalid_instruction)?;
    let case = cases[case_index];
    let storage_byte_len = activation.type_byte_len(variant.storage)?;

    // initialize deterministic bytes before projecting logical storage
    activation.zero_frame_bytes_at(instruction.a, byte_len);
    activation.copy_frame_bytes(
        instruction.c,
        instruction.a + case.payload_offset,
        storage_byte_len,
    );

    // encode the logical discriminant into the physical representation
    let field = variant.encoding.field();
    let field_offset = instruction.a + field.offset;
    let scalar = activation.read_frame_scalar(field_offset, field.byte_len as usize)?;
    let scalar = match variant.encoding {
        VariantEncoding::Direct { field } => field.insert(scalar, discriminant),
        encoding @ VariantEncoding::Niche { .. } => encoding
            .encode_niche(scalar, case_index as u32)
            .ok_or_else(Error::invalid_instruction)?,
    };
    activation.write_frame_scalar(field_offset, field.byte_len as usize, scalar)?;

    Ok(())
}

/// Load the logical discriminant from one physically encoded variant value.
pub(crate) fn execute_variant_discriminant(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (_, variant) = activation.variant_layout(instruction.c)?;
    let case = activation.active_variant_case(instruction.b, variant)?;
    let byte_len = activation.type_byte_len(variant.discriminant)?;

    activation.zero_frame_bytes_at(instruction.a, byte_len);
    activation.write_frame_scalar(instruction.a, byte_len, case.discriminant.bits())?;

    Ok(())
}

/// Load the logical payload storage from one physically encoded variant value.
pub(crate) fn execute_variant_storage(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (_, variant) = activation.variant_layout(instruction.c)?;
    let case = activation.active_variant_case(instruction.b, variant)?;
    let byte_len = activation.type_byte_len(variant.storage)?;

    activation.copy_frame_bytes(instruction.b + case.payload_offset, instruction.a, byte_len);

    Ok(())
}

impl Activation<'_> {
    /// Return one physical variant layout by raw layout id.
    fn variant_layout(&self, raw_layout: u32) -> Result<(usize, VariantLayout), Error> {
        let layout = self
            .program
            .layout_by_id(LayoutId::from_raw(raw_layout).ok_or_else(Error::invalid_instruction)?)
            .ok_or_else(Error::invalid_instruction)?;
        let LayoutShape::Variant(variant) = layout.shape else {
            return Err(Error::invalid_instruction());
        };

        Ok((layout.byte_len(), variant))
    }

    /// Return the active case for one physically encoded variant value.
    fn active_variant_case(
        &self,
        source: u32,
        variant: VariantLayout,
    ) -> Result<VariantCaseLayout, Error> {
        let field = variant.encoding.field();
        let scalar = self.read_frame_scalar(source + field.offset, field.byte_len as usize)?;
        let cases = self.program.variant_cases(variant);
        let case = match variant.encoding {
            VariantEncoding::Direct { field } => {
                let discriminant = field.extract(scalar);

                cases
                    .iter()
                    .find(|case| case.discriminant.bits() == discriminant)
            }
            encoding @ VariantEncoding::Niche { .. } => encoding
                .decode_niche(scalar)
                .and_then(|index| cases.get(index as usize)),
        };

        case.copied().ok_or_else(Error::invalid_instruction)
    }

    /// Return the byte width of one program type.
    fn type_byte_len(&self, ty: TypeId) -> Result<usize, Error> {
        self.program
            .layout(ty)
            .map(|layout| layout.byte_len())
            .ok_or_else(Error::invalid_instruction)
    }

    /// Read one little-endian scalar from frame bytes.
    fn read_frame_scalar(&self, offset: u32, byte_len: usize) -> Result<u128, Error> {
        let bytes = self.frame_bytes_at(offset, byte_len);
        let mut raw = [0; std::mem::size_of::<u128>()];
        let Some(destination) = raw.get_mut(..byte_len) else {
            return Err(Error::invalid_instruction());
        };
        destination.copy_from_slice(bytes);

        Ok(u128::from_le_bytes(raw))
    }

    /// Write one little-endian scalar into frame bytes.
    fn write_frame_scalar(
        &mut self,
        offset: u32,
        byte_len: usize,
        scalar: u128,
    ) -> Result<(), Error> {
        let scalar = scalar.to_le_bytes();
        let scalar = scalar
            .get(..byte_len)
            .ok_or_else(Error::invalid_instruction)?;

        self.store_frame_bytes_at(offset, scalar);

        Ok(())
    }
}
