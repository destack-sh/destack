use crate::{Diagnostic, DirChecked, bridge};

/// One checker output.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutput {
    /// Checked DIR artifact projection.
    pub checked: DirChecked,
    /// Diagnostics emitted by checking.
    pub diagnostics: Vec<Diagnostic>,
}
