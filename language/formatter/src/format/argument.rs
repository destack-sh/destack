use std::marker::PhantomData;

use destack_fir::format::{BestFittingMode, FormatResult};
use destack_workspace::TrailingComma;

use crate::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{Argument, LocalNodeId, Node, NodeTree, NodeTreeImpl, Parameter};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};

/// The kind of list, which affects trailing comma behavior.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum ListKind {
    /// Function parameters/arguments - trailing comma only with `All`.
    #[default]
    FunctionParameters,
    /// Collections (arrays, objects, tuples) - trailing comma with `All` or `Es5`.
    Collection,
}

impl ListKind {
    /// Whether a trailing comma should be added based on this kind and the option.
    pub(crate) fn should_add_trailing_comma(&self, option: TrailingComma) -> bool {
        match option {
            TrailingComma::All => true,
            TrailingComma::Es5 => matches!(self, ListKind::Collection),
            TrailingComma::None => false,
        }
    }
}

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
    kind: ListKind,
    elements: &'e Vec<LocalNodeId<T>>,

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

    /// Mark this as a collection (arrays, objects, tuples) for trailing comma purposes.
    pub(crate) fn as_collection(&mut self) -> &mut Self {
        self.kind = ListKind::Collection;
        self
    }
}

impl<'ast, 'e, T> Format<DestackFormatContext<'ast>> for ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        let options = &f.context().options;
        let trailing_comma_option = options.trailing_comma;
        let should_add_trailing = self.kind.should_add_trailing_comma(trailing_comma_option);
        let should_add_space = self.include_space && options.bracket_spacing;

        let body = &format_with(|f| {
            // leading space
            if should_add_space {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            // elements
            f.join_with(&format_args![
                &token(self.separator),
                soft_line_break_or_space()
            ])
            .entries(self.elements)
            .finish()?;

            // trailing separator
            if self.force_trailing_separator {
                write!(f, [token(self.separator)])?;
            } else if should_add_trailing {
                write!(f, [if_group_breaks(&token(self.separator))])?;
            }

            // trailing space
            if should_add_space && !self.elements.is_empty() {
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
///
/// By default, treats the list as function params for trailing comma purposes.
/// Call `.as_collection()` for arrays, objects, and tuples.
pub(crate) fn list_like<'ast, 'e, T>(
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    elements: &'e Vec<LocalNodeId<T>>,
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
        kind: ListKind::FunctionParameters,
        elements,
        _phantom: PhantomData,
    }
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<Parameter>,
        f: &mut DestackFormatter<'ast, '_>,
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
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Argument::Named { name, value } => {
                // name
                write!(f, [name])?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Labeled { label, value } => {
                // label
                write!(f, [label])?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { value } => {
                // value
                write!(f, [value])?;
            }
            Argument::Spread { value } => {
                // keyword
                write!(f, [token("...")])?;
                write!(f, [value])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_parameter() {
        assert_format!(
            "x: int32",
            "x: int32",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_with_default() {
        assert_format!(
            "x: int32 = 1",
            "x: int32 = 1",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named() {
        assert_format!(
            "x: 1",
            "x: 1",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named_shorthand() {
        assert_format!(
            "x",
            "x",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_positional() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }
}
