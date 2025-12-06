/// A variable represents a mutable binding from the source language.
///
/// During SSA construction, variables are mapped to SSA values. At control flow
/// join points, block parameters are inserted to merge different values.
///
/// This is distinct from MIR's `Local` which represents a stack slot.
/// Variables exist only during construction and are eliminated once SSA form
/// is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Variable(pub u32);

impl Variable {
    /// Create a new variable with the given id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the numeric id.
    pub fn id(&self) -> u32 {
        self.0
    }
}

impl From<u32> for Variable {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "var{}", self.0)
    }
}
