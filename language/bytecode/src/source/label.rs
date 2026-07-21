use std::fmt;

/// One function-local bytecode text label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Label(pub(crate) u32);

impl Label {
    /// Return this label's dense function-local index.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Label {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "l{}", self.0)
    }
}
