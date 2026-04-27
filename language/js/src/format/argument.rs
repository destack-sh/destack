use std::marker::PhantomData;

use crate::{Argument, GenericParameter, Keyword, LocalNodeId, Node, Parameter, Tree, TreeImpl};
use destack_fir::format::{BestFittingMode, FormatResult};

use crate::format::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{FormatNode, JsFormatContext, JsFormatter};

use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};

/// List like thing infix annotations.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    Tree: TreeImpl<T>,
{
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    include_space: bool,
    trailing_separator: TrailingSeparatorMode,
    force_expand: bool,
    elements: &'e Vec<LocalNodeId<T>>,

    _phantom: PhantomData<&'ast ()>,
}

/// The trailing separator policy for one list-like formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailingSeparatorMode {
    /// Never emit one trailing separator.
    Never,
    /// Emit one trailing separator only when the group breaks.
    Break,
    /// Always emit one trailing separator.
    Always,
}

#[allow(dead_code)]
impl<'ast, 'e, T> ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    Tree: TreeImpl<T>,
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
        self.trailing_separator = TrailingSeparatorMode::Always;
        self
    }

    pub(crate) fn without_trailing_separator(&mut self) -> &mut Self {
        self.trailing_separator = TrailingSeparatorMode::Never;
        self
    }
}

impl<'ast, 'e, T> Format<JsFormatContext<'ast>> for ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    Tree: TreeImpl<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, JsFormatContext<'ast>>) -> FormatResult<()> {
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

            // trailing separator
            match self.trailing_separator {
                TrailingSeparatorMode::Never => {}
                TrailingSeparatorMode::Break => {
                    write!(f, [if_group_breaks(&token(self.separator))])?;
                }
                TrailingSeparatorMode::Always => {
                    write!(f, [token(self.separator)])?;
                }
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
    elements: &'e Vec<LocalNodeId<T>>,
) -> ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    Tree: TreeImpl<T>,
{
    ListLike {
        start_token,
        end_token,
        separator,
        include_space: false,
        trailing_separator: TrailingSeparatorMode::Break,
        force_expand: false,
        elements,
        _phantom: PhantomData,
    }
}

/// Format one type parameter.
pub(crate) fn format_type_parameter<'ast>(
    parameter: &GenericParameter,
    f: &mut JsFormatter<'ast, '_>,
) -> FormatResult<()> {
    match parameter {
        GenericParameter::Type {
            modifiers,
            name,
            constraint,
            default,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [name])?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;

            if f.context().include_types()
                && let Some(constraint) = constraint
            {
                write!(f, [space(), Keyword::Extends, space(), constraint])?;
            }

            if let Some(default) = default {
                write!(f, [space(), token("="), space(), default])?;
            }
        }
    }

    Ok(())
}

/// Format one type parameter list.
pub(crate) fn format_type_parameter_list<'ast>(
    parameters: &[LocalNodeId<GenericParameter>],
    f: &mut JsFormatter<'ast, '_>,
) -> FormatResult<()> {
    write!(f, [token("<")])?;

    for (index, parameter_id) in parameters.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        let parameter = f.context().tree.get(*parameter_id);
        format_type_parameter(parameter, f)?;
    }

    write!(f, [token(">")])
}

impl<'ast> FormatNode<'ast, GenericParameter> for GenericParameter {
    fn format_node(
        &self,
        _node_id: LocalNodeId<GenericParameter>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_type_parameter(self, f)
    }
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Parameter>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
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
                if f.context().include_types()
                    && let Some(ty) = ty
                {
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
                if f.context().include_types()
                    && let Some(ty) = ty
                {
                    write!(f, [token(":"), space(), ty])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Parameter::VariadicNamed {
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
                if f.context().include_types()
                    && let Some(ty) = ty
                {
                    write!(f, [token(":"), space(), ty])?;
                }
            }
            Parameter::VariadicPattern {
                modifiers,
                pattern,
                ty,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [token("...")])?;
                // pattern
                write!(f, [pattern])?;
                // type
                if f.context().include_types()
                    && let Some(ty) = ty
                {
                    write!(f, [token(":"), space(), ty])?;
                }
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Argument>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Argument::Positional { value } => {
                write!(f, [value])?;
            }
            Argument::Spread { value } => {
                write!(f, [token("..."), value])?;
            }
        }
        Ok(())
    }
}
