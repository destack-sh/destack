use std::marker::PhantomData;

use dyst_fir::format::{BestFittingMode, FormatResult};

use crate::{
    Argument, DystFormatContext, DystFormatter, FormatNode, Node, NodeId, NodeTree, NodeTreeStore,
    Parameter,
};
use dyst_fir::prelude::*;
use dyst_fir::{best_fitting, format_args, write};

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
    force_trailing_separator: bool,
    force_expand: bool,
    elements: &'e Vec<NodeId<T>>,

    _phantom: PhantomData<&'ast ()>,
}

#[allow(dead_code)]
impl<'ast, 'e, T> ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeStore<T>,
{
    pub(crate) fn force_expand(&mut self) -> &mut Self {
        self.force_expand = true;
        self
    }

    pub(crate) fn should_expand(&mut self, should_expand: bool) -> &mut Self {
        self.force_expand = should_expand;
        self
    }

    pub(crate) fn include_space(&mut self) -> &mut Self {
        self.include_space = true;
        self
    }

    pub(crate) fn force_trailing_separator(&mut self) -> &mut Self {
        self.force_trailing_separator = true;
        self
    }
}

impl<'ast, 'e, T> Format<DystFormatContext<'ast>> for ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeStore<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        let body = &format_with(|f| {
            // leading space
            if self.include_space {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            // elements
            f.join_with(&format_args![
                &token(self.separator),
                soft_line_break_or_space()
            ])
            .entries(self.elements)
            .finish()?;

            // trailing separator (always if forced, otherwise only if group breaks)
            if self.force_trailing_separator {
                write!(f, [token(self.separator)])?;
            } else {
                write!(f, [if_group_breaks(&token(self.separator))])?;
            }

            // trailing space
            if self.include_space && !self.elements.is_empty() {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            Ok(())
        });

        let format_inline =
            format_with(|f| write!(f, [&token(self.start_token), body, &token(self.end_token)]));
        let format_indented = format_with(|f| {
            group(&format_args![
                &token(self.start_token),
                block_indent(body),
                &token(self.end_token)
            ])
            .should_expand(self.force_expand)
            .format(f)
        });

        if self.force_expand {
            format_indented.format(f)?;
        } else {
            best_fitting![format_inline, format_indented]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
        }

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
        include_space: false,
        force_trailing_separator: false,
        force_expand: false,
        elements,
        _phantom: PhantomData,
    }
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: NodeId<Parameter>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Parameter::Named { name, ty, default } => {
                // name
                write!(f, [name])?;
                // type
                if let Some(ty) = ty {
                    write!(f, [token(":"), space(), ty])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Parameter::Pattern {
                pattern,
                ty,
                default,
            } => {
                write!(f, [pattern])?;
                // type
                if let Some(ty) = ty {
                    write!(f, [token(":"), space(), ty])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Parameter::Variadic { name, ty } => {
                write!(f, [token("...")])?;
                // name
                write!(f, [name])?;
                // type
                if let Some(ty) = ty {
                    write!(f, [token(":"), space(), ty])?;
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: NodeId<Argument>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Argument::Named { name, value } => {
                write!(f, [name, token(":"), space(), value])?;
            }
            Argument::Shorthand { name } => {
                write!(f, [name])?;
            }
            Argument::Function { name, value } => {
                write!(f, [name, token(":"), space(), value])?;
            }
            Argument::Positional { value } => {
                write!(f, [value])?;
            }
            Argument::Spread { name, value } => {
                write!(f, [token("...")])?;
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                write!(f, [value])?;
            }
            Argument::Dynamic { name, key, value } => {
                write!(f, [token("[")])?;
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                write!(f, [key, token("]"), token(":"), space(), value])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
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
