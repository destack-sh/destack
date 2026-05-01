/// Static metadata about one diagnostic variant.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticDefinition {
    /// The full diagnostic code.
    pub code: &'static str,
    /// The variant name.
    pub name: &'static str,
    /// The doc comment description.
    pub description: &'static str,
    /// The numeric diagnostic sub-code.
    pub sub_code: u16,
}
