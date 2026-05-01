use crate::Word;

/// Constant value stored in lowered instructions.
#[derive(Clone, Debug)]
pub(crate) enum ConstValue {
    /// Pre-decoded constant value.
    Word(Word),
    /// Constant bytes for a frame-backed value.
    Bytes(Box<[u8]>),
}
