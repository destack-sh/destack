use crate::{FormatNode, JavaScriptFormatter};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};
use dyst_javascript_ast::{Keyword, Mutability, NodeId, Statement};

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        node_id: NodeId<Statement>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Statement::Block { block } => block.format(f)?,

            Statement::Let {
                mutability,
                pattern,
                ty,
                value,
            } => {
                match mutability {
                    Mutability::Mutable => write!(f, [Keyword::Let])?,
                    Mutability::Immutable => write!(f, [Keyword::Const])?,
                }
                write!(f, [pattern])?;
                if f.context().include_types()
                    && let Some(ty) = ty
                {
                    write!(f, [space(), token(":"), space(), ty])?;
                }
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), *value])?;
                }
            }
            Statement::LetType {
                name,
                static_parameters,
                value,
            } => {
                assert!(f.context().include_types());
                write!(f, [Keyword::Type, space(), name])?;
                if let Some(static_parameters) = static_parameters {
                    write!(
                        f,
                        [
                            token("<"),
                            format_with(|f| f
                                .join_with(&format_args![&token(","), space()])
                                .entries(static_parameters)
                                .finish()),
                            token(">")
                        ]
                    )?;
                }
                write!(f, [space(), token("="), space(), *value])?;
            }
            Statement::Assign {
                left,
                operator,
                right,
            } => {
                write!(f, [left, space(), operator, space(), right])?;
            }

            _ => todo!("format_node{self:?}"),
        }

        Ok(())
    }
}
