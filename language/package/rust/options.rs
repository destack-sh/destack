use dyst_fir::format::{IndentStyle, LineEnding};

/// The mode we're parsing, compiling, checking Dyst in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageMode {
    /// Lenient scripting with relaxed checking, conversion, cloning, boxing and more.
    Script,
    /// Strict engineering with explicit context, defaults, typing, behavior and more.
    Serious,
}

/// The options for working with the Dyst language.
#[derive(Debug, Clone, Default)]
pub struct LanguageOptions {
	/// The mode we're operating Dyst in.
	pub mode: LanguageMode = LanguageMode::Serious,
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}
