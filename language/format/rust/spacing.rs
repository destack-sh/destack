use std::num::NonZeroU8;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Indentation {
    /// Indent the content by `count` levels by using the indentation sequence specified by the printer options.
    Level(u16),

    /// Indent the content by n-`level`s using the indentation sequence specified by the printer options and `align` spaces.
    Align { level: u16, align: NonZeroU8 },
}

impl Indentation {
    pub(crate) const fn is_empty(self) -> bool {
        matches!(self, Indentation::Level(0))
    }

    /// Creates a new indentation level with a zero-indent.
    pub(crate) const fn new() -> Self {
        Indentation::Level(0)
    }

    /// Returns the indentation level
    pub(crate) fn level(self) -> u16 {
        match self {
            Indentation::Level(count) => count,
            Indentation::Align { level: indent, .. } => indent,
        }
    }

    /// Returns the number of trailing align spaces or 0 if none
    pub(crate) fn align(self) -> u8 {
        match self {
            Indentation::Level(_) => 0,
            Indentation::Align { align, .. } => align.into(),
        }
    }

    /// Increments the level by one.
    ///
    /// The behaviour depends on the [`indent_style`][IndentStyle] if this is an [`Indent::Align`]:
    /// - **Tabs**: `align` is converted into an indent. This results in `level` increasing by two: once for the align, once for the level increment
    /// - **Spaces**: Increments the `level` by one and keeps the `align` unchanged.
    ///   Keeps any  the current value is [`Indent::Align`] and increments the level by one.
    pub(crate) fn increment_level(self, indent_style: IndentStyle) -> Self {
        match self {
            Indentation::Level(count) => Indentation::Level(count + 1),
            // Increase the indent AND convert the align to an indent
            Indentation::Align { level, .. } if indent_style.is_tab() => {
                Indentation::Level(level + 2)
            }
            Indentation::Align {
                level: indent,
                align,
            } => Indentation::Align {
                level: indent + 1,
                align,
            },
        }
    }

    /// Decrements the indent by one by:
    /// - Reducing the level by one if this is [`Indent::Level`]
    /// - Removing the `align` if this is [`Indent::Align`]
    ///
    /// No-op if the level is already zero.
    pub(crate) fn decrement(self) -> Self {
        match self {
            Indentation::Level(level) => Indentation::Level(level.saturating_sub(1)),
            Indentation::Align { level, .. } => Indentation::Level(level),
        }
    }

    /// Adds an `align` of `count` spaces to the current indentation.
    ///
    /// It increments the `level` value if the current value is [`Indent::IndentAlign`].
    pub(crate) fn set_align(self, count: NonZeroU8) -> Self {
        match self {
            Indentation::Level(indent_count) => Indentation::Align {
                level: indent_count,
                align: count,
            },

            // Convert the existing align to an indent
            Indentation::Align { level: indent, .. } => Indentation::Align {
                level: indent + 1,
                align: count,
            },
        }
    }
}

impl Default for Indentation {
    fn default() -> Self {
        Indentation::new()
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, Default)]
pub enum IndentStyle {
    /// Use tabs to indent code.
    #[default]
    Tab,
    /// Use [`IndentWidth`] spaces to indent code.
    Space,
}

impl IndentStyle {
    /// Returns `true` if this is an [`IndentStyle::Tab`].
    pub const fn is_tab(&self) -> bool {
        matches!(self, IndentStyle::Tab)
    }

    /// Returns `true` if this is an [`IndentStyle::Space`].
    pub const fn is_space(&self) -> bool {
        matches!(self, IndentStyle::Space)
    }

    /// Returns the string representation of the indent style.
    pub const fn as_str(&self) -> &'static str {
        match self {
            IndentStyle::Tab => "tab",
            IndentStyle::Space => "space",
        }
    }
}

impl std::fmt::Display for IndentStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
