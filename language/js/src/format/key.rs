use crate::{ClassElementName, ModuleExportName, PropertyName};
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Context, Formatter};

impl<'ast> Format<'ast, Context<'ast>> for PropertyName {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            PropertyName::Identifier(name) => {
                write!(f, [name])?;
            }
            PropertyName::String(name) => {
                write!(f, [name])?;
            }
            PropertyName::Computed(expression) => {
                write!(f, [token("[")])?;
                write!(f, [expression])?;
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
