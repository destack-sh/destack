use std::fmt::Debug;

use super::{Format, FormatResult, Formatter};

/// A convenience wrapper for representing a formattable argument.
pub struct Argument<'fmt, 'a, Context> {
    value: &'fmt dyn Format<'a, Context>,
}

impl<Context> Debug for Argument<'_, '_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Argument").finish()
    }
}

impl<Context> Clone for Argument<'_, '_, Context> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Context> Copy for Argument<'_, '_, Context> {}

impl<'fmt, 'a, Context> Argument<'fmt, 'a, Context> {
    /// Called by the [tspp_fir::format_args] macro.
    #[doc(hidden)]
    #[inline]
    pub const fn new<F: Format<'a, Context>>(value: &'fmt F) -> Self {
        Self { value }
    }

    /// Format the value stored by this argument using the given formatter.
    #[inline]
    pub(super) fn format(&self, f: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        self.value.format(f)
    }
}

/// Sequence of objects that should be formatted in the specified order.
///
/// The [`format_args!`] macro will safely create an instance of this structure.
/// You can use the `Arguments<a>` that [`format_args!`] return in `Format` context as seen below.
/// It will call the `format` function for each of its objects.
pub struct Arguments<'fmt, 'a, Context>(pub &'fmt [Argument<'fmt, 'a, Context>]);

impl<'fmt, 'a, Context> Arguments<'fmt, 'a, Context> {
    #[doc(hidden)]
    #[inline]
    pub const fn new(arguments: &'fmt [Argument<'fmt, 'a, Context>]) -> Self {
        Self(arguments)
    }

    /// Get the arguments.
    #[inline]
    pub(super) fn items(&self) -> &'fmt [Argument<'fmt, 'a, Context>] {
        self.0
    }
}

impl<Context> Copy for Arguments<'_, '_, Context> {}

impl<Context> Clone for Arguments<'_, '_, Context> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, Context> Format<'a, Context> for Arguments<'_, 'a, Context> {
    #[inline]
    fn format(&self, formatter: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        formatter.write_format(*self)
    }
}

impl<Context> std::fmt::Debug for Arguments<'_, '_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Arguments[...]")
    }
}

impl<'fmt, 'a, Context> From<&'fmt Argument<'fmt, 'a, Context>> for Arguments<'fmt, 'a, Context> {
    fn from(argument: &'fmt Argument<'fmt, 'a, Context>) -> Self {
        Arguments::new(std::slice::from_ref(argument))
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{FormatState, Formatted, group};
    use crate::prelude::*;
    use crate::{format_args, write};

    /// Format nested arguments and verify the output structure.
    #[test]
    fn test_nesting() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_tspp(), &allocator);
        let mut formatter = Formatter::new(&mut state);

        write!(
            &mut formatter,
            [
                token("function"),
                space(),
                token("a"),
                space(),
                group(&format_args!(token("("), token(")")))
            ]
        )
        .unwrap();

        let instructions = formatter.into_tape().into_slice();
        let (context, groups, fits_expanded) = state.finish();
        let document = Document::new(instructions, groups, fits_expanded);
        let formatted = Formatted::new(document, context);

        assert_eq!(formatted.print().unwrap().as_str(), "function a ()");
    }
}
