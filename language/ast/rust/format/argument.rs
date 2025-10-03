use std::marker::PhantomData;

use dyst_fir::format::FormatResult;

use crate::{
    Argument, DystFormatContext, DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeStore,
    Parameter,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

/// List like thing infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeStore<T>,
{
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    include_space: bool,
    elements: &'e Vec<NodeId<T>>,

    _phantom: PhantomData<&'ast ()>,
}

impl<'ast, 'e, T> Format<DystFormatContext<'ast>> for ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeStore<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        write!(
            f,
            [group(&format_args![
                token(self.start_token),
                soft_block_indent(&format_with(|f| {
                    if self.include_space {
                        write!(f, [if_group_fits_on_line(&space())])?;
                    }

                    f.join_with(&format_args![
                        if_group_fits_on_line(&token(self.separator)),
                        soft_line_break_or_space()
                    ])
                    .entries(self.elements)
                    .finish()?;

                    if self.include_space && !self.elements.is_empty() {
                        write!(f, [if_group_fits_on_line(&space())])?;
                    }

                    Ok(())
                })),
                token(self.end_token)
            ]),]
        )?;

        Ok(())
    }
}

/// List like group for `elements`:
///  - beginning with `start_token`
///  - ending with `end_token`
///  - separated by `separator`
pub(crate) fn list_like<'ast, 'e, T>(
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    include_space: bool,
    elements: &'e Vec<NodeId<T>>,
) -> ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeStore<T>,
{
    ListLike {
        start_token,
        end_token,
        separator,
        include_space,
        elements,
        _phantom: PhantomData,
    }
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        _node_id: NodeId<Parameter>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(_node_id)])?;

        // name
        write!(f, [self.name])?;

        // type
        if let Some(r#type) = self.r#type {
            write!(f, [token(": "), r#type])?;
        }

        // default
        if let Some(default) = self.default {
            write!(f, [token(" = "), default])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(_node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        _node_id: NodeId<Argument>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(_node_id)])?;

        match self {
            Argument::Named { name, value } => {
                write!(f, [name, token(": "), value])?;
            }
            Argument::NamedShorthand { name } => {
                write!(f, [name])?;
            }
            Argument::Positional { value } => {
                write!(f, [value])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(_node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_parameter() {
        assert_format!(
            "x: int32",
            "x: int32",
            |p| p.eat_parameter(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_with_default() {
        assert_format!(
            "x: int32 = 1",
            "x: int32 = 1",
            |p| p.eat_parameter(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named() {
        assert_format!(
            "x: 1",
            "x: 1",
            |p| p.eat_argument(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named_shorthand() {
        assert_format!("x", "x", |p| p.eat_argument(), DystFormatOptions::default());
    }

    #[test]
    fn test_format_argument_positional() {
        assert_format!("1", "1", |p| p.eat_argument(), DystFormatOptions::default());
    }
}
