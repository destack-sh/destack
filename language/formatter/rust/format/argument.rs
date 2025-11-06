use std::marker::PhantomData;

use dyst_fir::format::{BestFittingMode, FormatResult};

use crate::variant::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{
    Argument, FormatNode, LanguageFormatContext, LanguageFormatter, Node, NodeId, NodeTree,
    NodeTreeImpl, Parameter,
};
use dyst_fir::prelude::*;
use dyst_fir::{best_fitting, format_args, write};

/// List like thing infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
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
    NodeTree: NodeTreeImpl<T>,
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

impl<'ast, 'e, T> Format<LanguageFormatContext<'ast>> for ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, LanguageFormatContext<'ast>>) -> FormatResult<()> {
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

        // prefer keeping the list on a single line
        let format_inline =
            format_with(|f| write!(f, [&token(self.start_token), body, &token(self.end_token)]));
        // otherwise, indent the body
        let format_indented = format_with(|f| {
            group(&format_args![
                &token(self.start_token),
                block_indent(body),
                &token(self.end_token)
            ])
            .should_expand(true)
            .format(f)
        });
        // if overall better fit, expand without indenting the
        let format_inline_expanded = format_with(|f| {
            write!(
                f,
                [
                    &token(self.start_token),
                    fits_expanded(&group(body).should_expand(true)),
                    &token(self.end_token)
                ]
            )
        });

        if self.force_expand {
            format_indented.format(f)?;
        } else {
            best_fitting![format_inline, format_indented, format_inline_expanded]
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
    NodeTree: NodeTreeImpl<T>,
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
        f: &mut LanguageFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
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
                modifiers,
                pattern,
                ty,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // pattern
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
            Parameter::Variadic {
                modifiers,
                name,
                ty,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
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
        f: &mut LanguageFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Argument::Named {
                modifiers,
                name,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Shorthand { modifiers, name } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
            Argument::Positional { modifiers, value } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // value
                write!(f, [value])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
            Argument::Spread {
                modifiers,
                name,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [token("...")])?;
                // name
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                write!(f, [value])?;
            }
            Argument::Dynamic {
                modifiers,
                name,
                key,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [token("[")])?;
                // name
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                write!(f, [key, token("]")])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Function {
                modifiers,
                name,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::DynamicFunction {
                modifiers,
                name,
                key,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [token("[")])?;
                // name
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                write!(f, [key, token("]")])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{LanguageFormatOptions, assert_format};

    #[test]
    fn test_format_parameter() {
        assert_format!(
            "x: int32",
            "x: int32",
            |p| p.eat_parameter(),
            LanguageFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_with_default() {
        assert_format!(
            "x: int32 = 1",
            "x: int32 = 1",
            |p| p.eat_parameter(),
            LanguageFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named() {
        assert_format!(
            "x: 1",
            "x: 1",
            |p| p.eat_argument(),
            LanguageFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named_shorthand() {
        assert_format!(
            "x",
            "x",
            |p| p.eat_argument(),
            LanguageFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_positional() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_argument(),
            LanguageFormatOptions::default()
        );
    }
}
