/// Options used while lowering checked DIR to MIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LowerOptions {
    /// Verify MIR while building it.
    pub verify_mir: bool,
}

impl Default for LowerOptions {
    fn default() -> Self {
        Self { verify_mir: true }
    }
}
