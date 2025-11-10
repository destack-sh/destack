use dyst_ast::StringPool;
use dyst_fir::format::{FormatContext, FormatOptions};
use dyst_fir::print::PrintOptions;
use dyst_javascript_ast::NodeTree;
use dyst_source::{File, IndentStyle, LineEnding};

use crate::{TranspilerLanguage, TranspilerUnit};

/// The formatting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatMode {
    /// Pretty.
    Pretty,
    /// Minimal.
    Minimal,
}

/// JS/TS format options (mostly for testing).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct JavaScriptFormatOptions {
    /// The formatting mode.
    pub mode: FormatMode = FormatMode::Pretty,
    /// The language target.
    pub language: TranspilerLanguage = TranspilerLanguage::TypeScript,
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl JavaScriptFormatOptions {
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

    /// Minimal options.
    pub fn minimal() -> Self {
        Self {
            mode: FormatMode::Minimal,
            indent_width: 0,
            ..Self::default()
        }
    }

    /// Minimal options with a given line width.
    pub fn minimal_with_line_width(line_width: u8) -> Self {
        Self {
            mode: FormatMode::Minimal,
            line_width,
            ..Self::default()
        }
    }

    /// Minimal options with tab indent style.
    pub fn minimal_tab() -> Self {
        Self {
            mode: FormatMode::Minimal,
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Set the language target.
    pub fn with_language(mut self, language: TranspilerLanguage) -> Self {
        self.language = language;
        self
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
    #[inline]
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }
}

impl FormatOptions for JavaScriptFormatOptions {
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

    #[inline]
    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// JS/TS format context.
#[derive(Debug, Clone)]
pub struct JavaScriptFormatContext<'a> {
    /// The format options.
    pub options: JavaScriptFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The unit.
    pub unit: &'a TranspilerUnit,
    /// The tree.
    pub tree: &'a NodeTree,
    /// The string pool.
    pub strings: &'a StringPool,
}

impl<'ast> JavaScriptFormatContext<'ast> {}

impl<'a> FormatContext for JavaScriptFormatContext<'a> {
    type Options = JavaScriptFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
    fn file(&self) -> &File {
        &self.file
    }
}
