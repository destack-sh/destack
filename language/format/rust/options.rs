use crate::{IndentStyle, IndentWidth, LineWidth, PrintOptions};

pub trait FormatOptions {
    /// The indent style.
    fn indent_style(&self) -> IndentStyle;

    /// The visual width of an indent
    fn indent_width(&self) -> IndentWidth;

    /// What's the max width of a line. Defaults to 80.
    fn line_width(&self) -> LineWidth;

    /// Derives the print options from these format options
    fn as_print_options(&self) -> PrintOptions;
}
