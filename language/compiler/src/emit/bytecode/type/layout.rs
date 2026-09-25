use tspp_mir as mir;

use crate::EmitError;

use super::TypeEmitter;

impl TypeEmitter<'_> {
    /// Return the concrete layout for one MIR type.
    pub(crate) fn layout(&self, ty: mir::TypeId) -> Result<&mir::Layout, EmitError> {
        self.optimized
            .layouts
            .type_layout(ty)
            .ok_or_else(|| self.internal("type has no optimized MIR layout"))
    }

    /// Return the exact runtime byte length of one MIR type.
    pub(crate) fn byte_len(&self, ty: mir::TypeId) -> Result<u32, EmitError> {
        Ok(self.layout(ty)?.size)
    }

    /// Return one source-ordered field layout.
    pub(crate) fn field(
        &self,
        ty: mir::TypeId,
        index: u32,
    ) -> Result<&mir::LayoutField, EmitError> {
        self.layout(ty)?
            .source_field(index)
            .ok_or_else(|| self.missing("field layout"))
    }

    /// Return one source-ordered aggregate placement.
    pub(crate) fn placement(&self, ty: mir::TypeId, index: u32) -> Result<(u32, u32), EmitError> {
        // read the aggregate layout
        let layout = self.layout(ty)?;

        // project named and positional fields by source order
        if let Some(field) = layout.source_field(index) {
            return Ok((field.offset, field.size));
        }

        // project fixed indexed storage by its physical stride
        if layout.element().is_some() {
            return self.element(ty, index);
        }

        // place the newtype's backing value at byte zero
        if let mir::LayoutShape::Newtype(backing) = &layout.shape {
            if index != 0 {
                return Err(self.missing("newtype placement"));
            }
            let byte_len = self.byte_len(backing.backing_type)?;

            return Ok((0, byte_len));
        }

        Err(self.missing("aggregate placement"))
    }

    /// Return one fixed element's byte offset and stored byte length.
    pub(crate) fn element(&self, ty: mir::TypeId, index: u32) -> Result<(u32, u32), EmitError> {
        let element = self
            .layout(ty)?
            .element()
            .ok_or_else(|| self.missing("element layout"))?;
        let byte_offset = element.stride * index;
        let byte_len = self.byte_len(element.element)?;

        Ok((byte_offset, byte_len))
    }

    /// Return one variant case payload's byte offset and stored byte length.
    pub(crate) fn variant(&self, ty: mir::TypeId, case: u32) -> Result<(u32, u32), EmitError> {
        let mir::LayoutShape::Variant(layout) = &self.layout(ty)?.shape else {
            return Err(self.missing("variant layout"));
        };
        let case = layout
            .cases
            .get(case as usize)
            .ok_or_else(|| self.missing("variant case layout"))?;
        let byte_len = self.byte_len(case.ty)?;

        Ok((case.payload_offset, byte_len))
    }

    /// Return one variant case's logical discriminant bits.
    pub(crate) fn variant_discriminant(
        &self,
        ty: mir::TypeId,
        case: u32,
    ) -> Result<u128, EmitError> {
        let mir::LayoutShape::Variant(layout) = &self.layout(ty)?.shape else {
            return Err(self.missing("variant layout"));
        };
        let case = layout
            .cases
            .get(case as usize)
            .ok_or_else(|| self.missing("variant case layout"))?;

        Ok(case.discriminant.bits())
    }

    /// Return the register word width of one aggregate layout.
    pub(super) fn word_count(&self, ty: mir::TypeId) -> Result<u16, EmitError> {
        let layout = self.layout(ty)?;
        let words = layout.size.div_ceil(size_of::<u64>() as u32);

        u16::try_from(words).map_err(|_| self.unsupported_type())
    }
}
