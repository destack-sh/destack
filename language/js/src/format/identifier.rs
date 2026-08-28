use crate::{Identifier, IdentifierName, Keyword, format_attributed, format_identifier, text_name};
use destack_core::StringId;
use destack_fir::format::{Format, FormatResult, text};
use destack_fir::prelude::{space, token};
use destack_fir::write;

use crate::{Context, Formatter};

impl<'ast> Format<'ast, Context<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for Identifier {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_identifier(*self, f)
    }
}

impl<'ast> Format<'ast, Context<'ast>> for IdentifierName {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let text_name = text_name(self.text, f.context())?;

        format_attributed(self.provenance, Some(text_name), f, |f| self.text.format(f))
    }
}

/// Format one shorthand property, expanding a renamed binding.
pub(crate) fn format_shorthand<'ast>(
    identifier: Identifier,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let emitted_name = identifier.emitted_name(f.context().symbols);
    if emitted_name == identifier.original_name {
        identifier.format(f)
    } else {
        let key = IdentifierName {
            text: identifier.original_name,
            provenance: identifier.provenance,
        };

        write!(f, [key, token(":"), space(), identifier])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
