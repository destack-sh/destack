use std::fmt::Debug;

use super::{Buffer, Format, FormatResult, Formatter};

/// A convenience wrapper for representing a formattable argument.
pub struct Argument<'fmt, Context> {
    value: &'fmt dyn Format<Context>,
}

impl<Context> Debug for Argument<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Argument").finish()
    }
}

impl<Context> Clone for Argument<'_, Context> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Context> Copy for Argument<'_, Context> {}

impl<'fmt, Context> Argument<'fmt, Context> {
    /// Called by the [dyst_fir::format_args] macro.
    #[doc(hidden)]
    #[inline]
    pub const fn new<F: Format<Context>>(value: &'fmt F) -> Self {
        Self { value }
    }

    /// Format the value stored by this argument using the given formatter.
    #[inline]
    pub(super) fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        self.value.format(f)
    }
}

/// Sequence of objects that should be formatted in the specified order.
///
/// The [`format_args!`] macro will safely create an instance of this structure.
/// You can use the `Arguments<a>` that [`format_args!`] return in `Format` context as seen below.
/// It will call the `format` function for each of its objects.
pub struct Arguments<'fmt, Context>(pub &'fmt [Argument<'fmt, Context>]);

impl<'fmt, Context> Arguments<'fmt, Context> {
    #[doc(hidden)]
    #[inline]
    pub const fn new(arguments: &'fmt [Argument<'fmt, Context>]) -> Self {
        Self(arguments)
    }

    /// Get the arguments.
    #[inline]
    pub(super) fn items(&self) -> &'fmt [Argument<'fmt, Context>] {
        self.0
    }
}

impl<Context> Copy for Arguments<'_, Context> {}

impl<Context> Clone for Arguments<'_, Context> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Context> Format<Context> for Arguments<'_, Context> {
    #[inline]
    fn format(&self, formatter: &mut Formatter<'_, Context>) -> FormatResult<()> {
        formatter.write_format(*self)
    }
}

impl<Context> std::fmt::Debug for Arguments<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Arguments[...]")
    }
}

impl<'fmt, Context> From<&'fmt Argument<'fmt, Context>> for Arguments<'fmt, Context> {
    fn from(argument: &'fmt Argument<'fmt, Context>) -> Self {
        Arguments::new(std::slice::from_ref(argument))
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{FormatState, FormatTag, VecBuffer, group};
    use crate::prelude::*;
    use crate::{format_args, write};

    /// Format nested arguments and verify the output structure.
    #[test]
    fn test_nesting() {
        let mut context = FormatState::new(SimpleFormatContext::default());
        let mut buffer = VecBuffer::new(&mut context);

        write!(
            &mut buffer,
            [
                token("function"),
                space(),
                token("a"),
                space(),
                group(&format_args!(token("("), token(")")))
            ]
        )
        .unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![
                FormatNode::Token { text: "function" },
                FormatNode::Space,
                FormatNode::Token { text: "a" },
                FormatNode::Space,
                // Group
                FormatNode::Tag(FormatTag::StartGroup(group::Group::new())),
                FormatNode::Token { text: "(" },
                FormatNode::Token { text: ")" },
                FormatNode::Tag(FormatTag::EndGroup)
            ]
        );
    }
}
