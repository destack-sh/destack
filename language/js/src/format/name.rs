use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::expression::format_expression_id_with_precedence;
use crate::{ClassElementName, Context, Formatter, ModuleExportName, Precedence, PropertyName};

impl<'ast> Format<'ast, Context<'ast>> for PropertyName {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            PropertyName::Identifier(name) => write!(f, [name])?,
            PropertyName::String(name) => write!(f, [name])?,
            PropertyName::Computed(expression) => {
                write!(f, [token("[")])?;
                format_expression_id_with_precedence(*expression, Precedence::Assignment, f)?;
                write!(f, [token("]")])?;
            }
        }

        Ok(())
    }
}

impl<'ast> Format<'ast, Context<'ast>> for ClassElementName {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            ClassElementName::Public(name) => write!(f, [name]),
            ClassElementName::Private(identifier) => write!(f, [token("#"), identifier]),
        }
    }
}

impl<'ast> Format<'ast, Context<'ast>> for ModuleExportName {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            ModuleExportName::Identifier(name) => write!(f, [name]),
            ModuleExportName::String(name) => write!(f, [name]),
        }
    }
}
