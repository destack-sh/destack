use crate::BestFittingMode;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum MeasureMode {
    /// The content fits if a hard line break or soft line break in [`PrintMode::Expanded`] is seen before exceeding the configured print width.
    FirstLine,

    /// The content only fits if none of the lines exceed the print width.
    /// Lines are terminated by either a hard line break or a soft line break in [`PrintMode::Expanded`].
    AllLines,

    /// Measures all lines and allows lines to exceed the configured line width.
    /// Useful when it only matters whether the content *before* and *after* fits.
    AllLinesAllowTextOverflow,
}

impl MeasureMode {
    /// Check if this mode allows text exceeding the configured line width.
    pub(crate) const fn allows_text_overflow(self) -> bool {
        matches!(self, MeasureMode::AllLinesAllowTextOverflow)
    }
}

impl From<BestFittingMode> for MeasureMode {
    fn from(value: BestFittingMode) -> Self {
        match value {
            BestFittingMode::FirstLine => Self::FirstLine,
            BestFittingMode::AllLines => Self::AllLines,
        }
    }
}
