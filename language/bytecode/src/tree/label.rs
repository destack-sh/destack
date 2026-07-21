use std::fmt;

/// One dense function-local branch label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(pub u32);

impl Label {
    /// Return this label's dense function-local index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Label {
    /// Format this label in canonical bytecode text form.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "l{}", self.0)
    }
}
