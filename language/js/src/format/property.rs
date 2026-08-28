use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::expression::format_expression_id_with_precedence;
use crate::format::function::{format_function_parameters, format_method, format_static};
use crate::format::identifier::format_shorthand;
use crate::{FormatNode, Formatter, Keyword, Member, Precedence, Property};

impl<'ast> FormatNode<'ast> for Property {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Field { key, value } => {
                write!(f, [key, token(":"), space()])?;
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
            Self::Shorthand { value } => format_shorthand(*value, f)?,
            Self::Method {
                key,
                signature,
                body,
            } => {
                format_method(false, key, signature, f)?;
                write!(f, [space(), body])?;
            }
            Self::Getter { key, body } => {
                write!(
                    f,
                    [
                        Keyword::Get,
                        space(),
                        key,
                        token("("),
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            Self::Setter {
                key,
                parameter,
                body,
            } => {
                write!(
                    f,
                    [
                        Keyword::Set,
                        space(),
                        key,
                        token("("),
                        parameter,
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            Self::Spread { value } => {
                write!(f, [token("...")])?;
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for Member {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Field {
                key,
                default,
                is_static,
            } => {
                format_static(*is_static, f)?;
                write!(f, [key])?;
                if let Some(default) = default {
                    write!(f, [space(), token("="), space()])?;
                    format_expression_id_with_precedence(*default, Precedence::Assignment, f)?;
                }
                write!(f, [token(";")])?;
            }
            Self::Method {
                key,
                signature,
                body,
                is_static,
            } => {
                format_method(*is_static, key, signature, f)?;
                write!(f, [space(), body])?;
            }
            Self::Getter {
                key,
                body,
                is_static,
            } => {
                format_static(*is_static, f)?;
                write!(
                    f,
                    [
                        Keyword::Get,
                        space(),
                        key,
                        token("("),
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            Self::Setter {
                key,
                parameter,
                body,
                is_static,
            } => {
                format_static(*is_static, f)?;
                write!(
                    f,
                    [
                        Keyword::Set,
                        space(),
                        key,
                        token("("),
                        parameter,
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            Self::Constructor {
                parameters,
                rest,
                body,
            } => {
                write!(f, [Keyword::Constructor])?;
                format_function_parameters(parameters, *rest, f)?;
                write!(f, [space(), body])?;
            }
            Self::StaticBlock { body } => write!(f, [Keyword::Static, space(), body])?,
        }

        Ok(())
    }
}
