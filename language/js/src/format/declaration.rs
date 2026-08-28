use crate::format::expression::format_expression_id_with_precedence;
use crate::format::function::format_function_signature_parameters;
use crate::{Asynchrony, Declaration, FormatNode, Formatter, Keyword, Precedence};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast> for Declaration {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Class(class) => {
                // write the class header
                write!(f, [Keyword::Class])?;
                if let Some(name) = class.name {
                    write!(f, [space(), name])?;
                }
                if let Some(extends) = class.extends_expression {
                    write!(f, [space(), Keyword::Extends, space()])?;
                    format_expression_id_with_precedence(extends, Precedence::Call, f)?;
                }

                // write the class body
                if class.members.is_empty() {
                    write!(f, [space(), token("{"), token("}")])?;
                } else {
                    write!(f, [space(), token("{"), hard_line_break()])?;
                    write!(
                        f,
                        [block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(&class.members)
                            .finish()))]
                    )?;
                    write!(f, [hard_line_break(), token("}")])?;
                }
            }
            Self::Function(function) => {
                let signature = &function.signature;

                // write the function header
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }
                write!(f, [Keyword::Function])?;
                if signature.is_generator {
                    write!(f, [token("*")])?;
                }
                match function.name {
                    Some(name) => {
                        write!(f, [space(), name])?;
                        format_function_signature_parameters(signature, f)?;
                    }
                    None => {
                        format_function_signature_parameters(signature, f)?;
                    }
                }

                // write the function body
                write!(f, [space(), function.body])?;
            }
        }

        Ok(())
    }
}
