use crate::format::function::format_function_signature_parameters;
use crate::{
    Asynchrony, Context, Declaration, ExportKind, FormatNode, Formatter, Keyword, LocalNodeId,
};
use tspp_fir::format::{Format, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

impl<'ast> Format<'ast, Context<'ast>> for ExportKind {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Named => write!(f, [Keyword::Export]),
            Self::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
        }
    }
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Declaration>,
        f: &mut Formatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Self::Class(class) => {
                // write the class header
                if let Some(export) = class.export {
                    write!(f, [export, space()])?;
                }
                write!(f, [Keyword::Class])?;
                if let Some(name) = class.name {
                    write!(f, [space(), name])?;
                }
                if let Some(extends) = class.extends_expression {
                    write!(f, [space(), Keyword::Extends, space(), extends])?;
                }

                // write the class body
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
            Self::Function(function) => {
                let signature = &function.signature;

                // write the function header
                if let Some(export) = function.export {
                    write!(f, [export, space()])?;
                }
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }
                write!(f, [Keyword::Function])?;
                if signature.is_generator {
                    write!(f, [token("*")])?;
                }
                if let Some(name) = function.name {
                    write!(f, [space(), name])?;
                }
                format_function_signature_parameters(signature, f)?;

                // write the function body
                write!(f, [space(), function.body])?;
            }
        }

        Ok(())
    }
}
