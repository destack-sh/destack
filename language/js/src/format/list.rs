use destack_fir::format::{BestFittingMode, Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};

use crate::{Context, Formatter};

/// One delimited JavaScript list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DelimitedList<'element, T> {
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    include_space: bool,
    trailing_separator: TrailingSeparator,
    elements: &'element [T],
}

/// The trailing separator policy for one delimited list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailingSeparator {
    /// Never emit one trailing separator.
    Never,
    /// Always emit one trailing separator.
    Always,
    /// Emit one trailing separator only when the group breaks.
    IfBreaks,
}

impl<T> DelimitedList<'_, T> {
    /// Include spaces inside the delimiters when the list stays inline.
    pub(crate) fn include_space(&mut self) -> &mut Self {
        self.include_space = true;
        self
    }

    /// Never emit a trailing separator.
    pub(crate) fn without_trailing_separator(&mut self) -> &mut Self {
        self.trailing_separator = TrailingSeparator::Never;
        self
    }

    /// Always emit one trailing separator.
    pub(crate) fn with_trailing_separator(&mut self) -> &mut Self {
        self.trailing_separator = TrailingSeparator::Always;
        self
    }
}

impl<'ast, T> Format<'ast, Context<'ast>> for DelimitedList<'_, T>
where
    T: Format<'ast, Context<'ast>>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let body = &format_with(|f| {
            // leading space
            if self.include_space && !self.elements.is_empty() {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            // elements
            f.join_with(&format_args![
                &token(self.separator),
                soft_line_break_or_space()
            ])
            .entries(self.elements)
            .finish()?;

            // trailing separator
            match self.trailing_separator {
                TrailingSeparator::Never => {}
                TrailingSeparator::Always => write!(f, [token(self.separator)])?,
                TrailingSeparator::IfBreaks => {
                    write!(f, [if_group_breaks(&token(self.separator))])?;
                }
            }

            // trailing space
            if self.include_space && !self.elements.is_empty() {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            Ok(())
        });

        // prefer keeping the list on a single line
        let format_inline =
            format_with(|f| write!(f, [&token(self.start_token), body, &token(self.end_token)]));

        // otherwise, indent the body
        let format_indented = format_with(|f| {
            group(&format_args![
                &token(self.start_token),
                block_indent(body),
                &token(self.end_token)
            ])
            .should_expand(true)
            .format(f)
        });

        // otherwise expand the body without adding indentation
        let format_inline_expanded = format_with(|f| {
            write!(
                f,
                [
                    &token(self.start_token),
                    fits_expanded(&group(body).should_expand(true)),
                    &token(self.end_token)
                ]
            )
        });

        best_fitting![format_inline, format_indented, format_inline_expanded]
            .with_mode(BestFittingMode::AllLines)
            .format(f)?;

        Ok(())
    }
}

/// Format elements between tokens with one separator.
pub(crate) fn delimited<'element, T>(
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    elements: &'element [T],
) -> DelimitedList<'element, T> {
    DelimitedList {
        start_token,
        end_token,
        separator,
        include_space: false,
        trailing_separator: TrailingSeparator::IfBreaks,
        elements,
    }
}
