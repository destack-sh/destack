use destack_memory::AddressSpace;

/// The source bytes used to initialize one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload<'a> {
    /// Caller-provided bytes.
    Bytes(&'a [u8]),
    /// Zeroed bytes.
    Zeroed,
    /// Uninitialized bytes.
    Uninit,
}

impl<'a> Payload<'a> {
    /// Return the explicit payload byte length when this payload has one.
    pub(crate) fn byte_len(&self) -> Option<usize> {
        match self {
            Self::Bytes(bytes) => Some(bytes.len()),
            Self::Zeroed | Self::Uninit => None,
        }
    }

    /// Initialize one materialized mapped byte range from this payload.
    #[inline(always)]
    pub(crate) fn initialize_mapped(
        &self,
        mapping: &AddressSpace,
        offset: usize,
        clear_byte_len: usize,
    ) {
        match self {
            // clear the trailing slack before copying short payloads
            Self::Bytes(bytes) if bytes.len() < clear_byte_len => {
                // SAFETY: block paths materialize the destination before publishing it
                unsafe {
                    mapping.zero_mapped_bytes(offset, clear_byte_len);
                    mapping.write_mapped_bytes(offset, bytes);
                }
            }
            // full-width payloads overwrite the range completely
            Self::Bytes(bytes) => {
                // SAFETY: block paths materialize the destination before publishing it
                unsafe {
                    mapping.write_mapped_bytes(offset, bytes);
                }
            }
            // zeroed payloads only clear
            Self::Zeroed => {
                // SAFETY: block paths materialize the destination before publishing it
                unsafe {
                    mapping.zero_mapped_bytes(offset, clear_byte_len);
                }
            }
            // uninitialized payloads are owned by the caller until written
            Self::Uninit => {}
        }
    }

    /// Initialize one already-zeroed materialized mapped byte range from this payload.
    #[inline(always)]
    pub(crate) fn initialize_zeroed_mapped(&self, mapping: &AddressSpace, offset: usize) {
        if let Self::Bytes(bytes) = self {
            // SAFETY: block paths materialize the destination before publishing it
            unsafe {
                mapping.write_mapped_bytes(offset, bytes);
            }
        }
    }
}
