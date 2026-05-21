/// Estimated source bytes per parser token.
pub(crate) const ESTIMATED_TOKEN_BYTES: usize = 6;
/// Estimated parser tokens per interned string.
pub(crate) const ESTIMATED_STRING_TOKEN_DIVISOR: usize = 3;
/// Maximum nested recursive parser descent before reporting malformed input.
pub(crate) const MAX_RECURSIVE_DESCENT_DEPTH: u16 = 512;
