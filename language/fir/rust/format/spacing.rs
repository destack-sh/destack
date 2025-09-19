#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Indentation {
    /// Indent the content by `count` levels by using the indentation sequence specified by the printer options.
    Level(u16),

    /// Indent the content by n-`level`s using the indentation sequence specified by the printer options and `align` spaces.
    Align { level: u16, align: u8 },
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

    /// Decrement the indent by one.
    ///
    /// - Reducing the level by one if this is [`Indentation::Level`]
    /// - Removing the `align` if this is [`Indentation::Align`]
    ///
    /// No-op if the level is already zero.
    pub(crate) fn decrement(self) -> Self {
        match self {
            Indentation::Level(level) => Indentation::Level(level.saturating_sub(1)),
            Indentation::Align { level, .. } => Indentation::Level(level),
        }
    }

    /// Add an `align` of `count` spaces to the current indentation.
    ///
    /// It increments the `level` value if the current value is [`Indentation::Align`].
    pub(crate) fn set_align(self, count: u8) -> Self {
        match self {
            Indentation::Level(indent_count) => Indentation::Align {
                level: indent_count,
                align: count,
            },

            // convert the existing align to an indent
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
    /// Check if this is an [`IndentStyle::Tab`].
    pub const fn is_tab(&self) -> bool {
        matches!(self, IndentStyle::Tab)
    }

    /// Check if this is an [`IndentStyle::Space`].
    pub const fn is_space(&self) -> bool {
        matches!(self, IndentStyle::Space)
    }

    /// Get the string representation of the indent style.
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum LineEnding {
    /// Line Feed only (\n), common on Linux and macOS as well as inside git repos.
    #[default]
    LineFeed,
    /// Carriage Return + Line Feed characters (\r\n), common on Windows.
    CarriageReturnLineFeed,
    /// Carriage Return character only (\r), used very rarely.
    CarriageReturn,
}

impl LineEnding {
    /// Get the string representation of this line ending.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "\n",
            LineEnding::CarriageReturnLineFeed => "\r\n",
            LineEnding::CarriageReturn => "\r",
        }
    }

    /// Get the string used to configure this line ending.
    ///
    /// See [`LineEnding::as_str`] for the actual string representation of the line ending.
    #[inline]
    pub const fn as_setting_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "lf",
            LineEnding::CarriageReturnLineFeed => "crlf",
            LineEnding::CarriageReturn => "cr",
        }
    }
}
