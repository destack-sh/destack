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
}
