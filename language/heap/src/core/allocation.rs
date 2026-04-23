/// One allocation initializer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Allocation<'a> {
    /// Caller-provided bytes.
    Bytes(&'a [u8]),
    /// Zeroed bytes.
    Zeroed,
}

impl<'a> Allocation<'a> {
    /// Return the caller-provided bytes when this allocation has them.
    pub(crate) fn bytes(&self) -> Option<&'a [u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Zeroed => None,
        }
    }
}
