use dyst_fir::format::FormatOptions;
use dyst_fir::print::PrintOptions;
use dyst_source::{IndentStyle, LineEnding};

use crate::{EcmaScriptVersion, LanguageTarget, TypeScriptVersion};

/// The formatting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatMode {
    /// Pretty.
    Pretty,
    /// Minified.
    Minified,
}

/// JS/TS format options (mostly for testing).
#[derive(Debug, Default, PartialEq, Clone)]
pub struct LanguageFormatOptions {
    /// The formatting mode.
    pub mode: FormatMode = FormatMode::Pretty,
	/// The target language.
	pub target: LanguageTarget,
	/// The ECMAScript level.
	pub es_version: EcmaScriptVersion,
	/// The TypeScript version.
	pub ts_version: TypeScriptVersion,
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl LanguageFormatOptions {
    /// Pretty options.
    pub fn pretty() -> Self {
        Self {
            mode: FormatMode::Pretty,
            ..Self::default()
        }
    }

    /// Pretty options with a given line width.
    pub fn pretty_with_line_width(line_width: u8) -> Self {
        Self {
            mode: FormatMode::Pretty,
            line_width,
            ..Self::default()
        }
    }

    /// Default options with tab indent style.
    pub fn pretty_tab() -> Self {
        Self {
            mode: FormatMode::Pretty,
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Minified options.
    pub fn minified() -> Self {
        Self {
            mode: FormatMode::Minified,
            indent_width: 1,
            ..Self::default()
        }
    }

    /// Minified options with a given line width.
    pub fn minified_with_line_width(line_width: u8) -> Self {
        Self {
            mode: FormatMode::Minified,
            line_width,
            ..Self::default()
        }
    }

    /// Minified options with tab indent style.
    pub fn minified_tab() -> Self {
        Self {
            mode: FormatMode::Minified,
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Set the line ending.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    /// Convert to print options.
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }
}

impl FormatOptions for LanguageFormatOptions {
    #[inline]
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    #[inline]
    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    #[inline]
    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}
