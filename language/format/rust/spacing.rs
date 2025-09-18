use std::num::NonZeroU8;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Indentation {
    /// Indent the content by `count` levels by using the indentation sequence specified by the printer options.
    Level(u16),

    /// Indent the content by n-`level`s using the indentation sequence specified by the printer options and `align` spaces.
    Align { level: u16, align: NonZeroU8 },
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

/// The visual width of an indentation.
///
/// Determines the visual width of a tab character (`\t`) and the number of
/// spaces per indent when using [`IndentStyle::Space`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndentWidth(NonZeroU8);

impl IndentWidth {
    /// Return the numeric value for this [`LineWidth`]
    pub const fn value(&self) -> u32 {
        self.0.get() as u32
    }
}

impl Default for IndentWidth {
    fn default() -> Self {
        Self(NonZeroU8::new(2).unwrap())
    }
}

impl Display for IndentWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl TryFrom<u8> for IndentWidth {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        NonZeroU8::try_from(value).map(Self)
    }
}

impl From<NonZeroU8> for IndentWidth {
    fn from(value: NonZeroU8) -> Self {
        Self(value)
    }
}

/// The maximum visual width to which the formatter should try to limit a line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineWidth(NonZeroU16);

impl LineWidth {
    /// Return the numeric value for this [`LineWidth`]
    pub const fn value(&self) -> u16 {
        self.0.get()
    }
}

impl Default for LineWidth {
    fn default() -> Self {
        Self(NonZeroU16::new(80).unwrap())
    }
}

impl Display for LineWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<LineWidth> for u16 {
    fn from(value: LineWidth) -> Self {
        value.0.get()
    }
}

impl From<LineWidth> for u32 {
    fn from(value: LineWidth) -> Self {
        u32::from(value.0.get())
    }
}

impl From<NonZeroU16> for LineWidth {
    fn from(value: NonZeroU16) -> Self {
        Self(value)
    }
}
