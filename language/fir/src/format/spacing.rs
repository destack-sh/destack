#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Indentation {
    /// Indent the content by `count` levels by using the indentation sequence specified by the printer options.
    Level(u16),

    /// Indent the content by `level`s plus `align` spaces.
    Align {
        level: u16,
        align: u8,
        align_depth: u16,
    },
}

impl Indentation {
    pub(crate) const fn is_empty(self) -> bool {
        matches!(self, Indentation::Level(0))
    }

    /// Create a new indentation level with a zero-indent.
    pub(crate) const fn new() -> Self {
        Indentation::Level(0)
    }

    /// Get the indentation level.
    pub(crate) fn level(self) -> u16 {
        match self {
            Indentation::Level(count) => count,
            Indentation::Align { level: indent, .. } => indent,
        }
    }

    /// Get the number of trailing align spaces or 0 if none.
    pub(crate) fn align(self) -> u8 {
        match self {
            Indentation::Level(_) => 0,
            Indentation::Align { align, .. } => align,
        }
    }

    /// Increment the level by one.
    ///
    /// The behaviour depends on the [`indent_style`][IndentStyle] if this is an [`Indentation::Align`]:
    /// - **Tabs**: `align` is converted into an indent.
    ///   This results in `level` increasing by two: once for the align, once for the level increment
    /// - **Spaces**: Increments the `level` by one and keeps the `align` unchanged.
    pub(crate) fn increment_level(self, indent_style: IndentStyle) -> Self {
        match self {
            Indentation::Level(count) => Indentation::Level(count + 1),
            // increase the indent AND convert the align to an indent
            Indentation::Align {
                level, align_depth, ..
            } if indent_style.is_tab() => Indentation::Level(level + align_depth + 1),
            Indentation::Align {
                level: indent,
                align,
                align_depth,
            } => Indentation::Align {
                level: indent + 1,
                align,
                align_depth,
            },
        }
    }

    /// Add an `align` of `count` spaces to the current indentation.
    ///
    /// Nested aligns accumulate their space widths on the current indentation.
    pub(crate) fn set_align(self, count: u8) -> Self {
        match self {
            Indentation::Level(indent_count) => Indentation::Align {
                level: indent_count,
                align: count,
                align_depth: 1,
            },

            // nested aligns accumulate without becoming full indent levels
            Indentation::Align {
                level,
                align,
                align_depth,
            } => Indentation::Align {
                level,
                align: align.saturating_add(count),
                align_depth: align_depth + 1,
            },
        }
    }
}

impl Default for Indentation {
    fn default() -> Self {
        Indentation::new()
    }
}

pub use destack_source::{IndentStyle, LineEnding};
