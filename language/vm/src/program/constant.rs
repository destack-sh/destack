/// Constant value stored in lowered instructions.
#[derive(Clone, Debug)]
pub(crate) enum ConstValue {
    /// Constant bytes for a frame-backed value.
    Bytes(Box<[u8]>),
}
