use dyst_fir::format::FormatResult;

use crate::r#let::FormatScopedMutability;
use crate::{DystFormatter, FormatNode, Mutability, NodeId, ScopedMutability, Type};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        node_id: NodeId<Type>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Type::Infer => write!(f, [token("_")]),
            Type::Maybe(type_) => write!(f, [type_, token("?")]),
            Type::Not(type_) => write!(f, [token("!"), type_]),
            Type::Never => write!(f, [token("!")]),
            Type::Self_ => write!(f, [token("Self")]),
            Type::Primitive(primitive) => write!(f, [primitive]),
            Type::Path {
                path,
                static_arguments,
            } => {
                if let Some(arguments) = static_arguments {
                    write!(
                        f,
                        [
                            path,
                            group(&format_args![
                                token("<"),
                                soft_block_indent(&format_with(|f| f
                                    .join_with(&format_args![
                                        if_group_fits_on_line(&token(",")),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(arguments)
                                    .finish())),
                                token(">")
                            ])
                        ]
                    )
                } else {
                    write!(f, [path])
                }
            }
            Type::Reference { mutability, target } => {
                write!(f, [token("&")])?;
                write!(
                    f,
                    [FormatScopedMutability::implicit_const(mutability.clone()),]
                )?;
                match mutability {
                    ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    } => {}
                    _ => write!(f, [space()])?,
                }
                write!(f, [target])
            }
            Type::Virtual(target) => {
                write!(f, [token("$"), target])
            }
            Type::Variadic(target) => {
                write!(f, [token(".."), target])
            }
            Type::Array { element, count } => {
                write!(f, [element, token("["), count, token("]")])
            }
            Type::Slice { element } => {
                write!(f, [element, token("[]")])
            }
            Type::Tuple(tuple) => write!(f, [tuple]),
            Type::InlineStruct(struct_) => write!(f, [struct_]),
            Type::InlineEnum(enum_) => write!(f, [enum_]),
            Type::InlineUnion(union) => write!(f, [union]),
            Type::Union(types) => write!(
                f,
                [format_with(|f| f
                    .join_with(&token(" | "))
                    .entries(types)
                    .finish())]
            ),
            Type::Intersection(types) => write!(
                f,
                [format_with(|f| f
                    .join_with(&token(" & "))
                    .entries(types)
                    .finish())]
            ),
            Type::Function(function) => write!(f, [function]),
        }?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}
