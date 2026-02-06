use std::collections::HashMap;
use std::marker::PhantomData;

use destack_fir::format::{BestFittingMode, FormatResult, GroupId};
use destack_workspace::TrailingComma;

use crate::directive::{
    collect_comment_tokens, ignore_range_for_node, ignored_span_source, write_ignored_span,
};
use crate::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Annotation, AnnotationPosition, Argument, Declaration, Expression, Keyword, LocalNodeId, Node,
    NodeTree, NodeTreeImpl, NodeType, Parameter,
};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};
use destack_source::Span;

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
    allow_trailing_separator: bool,
    force_expand: bool,
    kind: ListKind,
    group_id: Option<GroupId>,
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

    pub(crate) fn disallow_trailing_separator(&mut self) -> &mut Self {
        self.allow_trailing_separator = false;
        self
    }

    /// Mark this as a collection (arrays, objects, tuples) for trailing comma purposes.
    pub(crate) fn as_collection(&mut self) -> &mut Self {
        self.kind = ListKind::Collection;
        self
    }

    pub(crate) fn with_group_id(&mut self, group_id: Option<GroupId>) -> &mut Self {
        self.group_id = group_id;
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
        let should_add_trailing = self.allow_trailing_separator
            && self.kind.should_add_trailing_comma(trailing_comma_option);
        let has_elements = !self.elements.is_empty();
        let should_add_space = self.include_space && options.bracket_spacing && has_elements;

        let mut ignore_ranges_by_id = HashMap::new();
        let comment_tokens = collect_comment_tokens(f.context());
        for element_id in self.elements {
            if let Some(range_span) =
                ignore_range_for_node(f.context(), *element_id, &comment_tokens)
            {
                ignore_ranges_by_id.insert(element_id.id, range_span);
            }
        }
        let has_ignore_ranges = !ignore_ranges_by_id.is_empty();

        let body = &format_with(|f| {
            // leading space
            if should_add_space {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            let mut needs_trailing_separator = true;
            if has_ignore_ranges {
                needs_trailing_separator = format_list_with_ignored_ranges(
                    f,
                    self.elements,
                    &ignore_ranges_by_id,
                    self.separator,
                )?;
            } else {
                // elements
                f.join_with(&format_args![
                    &token(self.separator),
                    soft_line_break_or_space()
                ])
                .entries(self.elements)
                .finish()?;
            }

            // trailing separator
            if self.force_trailing_separator
                && self.allow_trailing_separator
                && (!has_ignore_ranges || needs_trailing_separator)
            {
                write!(f, [token(self.separator)])?;
            } else if should_add_trailing && (!has_ignore_ranges || needs_trailing_separator) {
                write!(f, [if_group_breaks(&token(self.separator))])?;
            }

            // trailing space
            if should_add_space {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            Ok(())
        });

        // prefer keeping the list on a single line
        let format_inline = format_with(|f| {
            if let Some(group_id) = self.group_id {
                group(&format_args![
                    &token(self.start_token),
                    body,
                    &token(self.end_token)
                ])
                .with_id(Some(group_id))
                .format(f)
            } else {
                write!(f, [&token(self.start_token), body, &token(self.end_token)])
            }
        });

        // otherwise, indent the body
        let format_indented = format_with(|f| {
            group(&format_args![
                &token(self.start_token),
                block_indent(body),
                &token(self.end_token)
            ])
            .with_id(self.group_id)
            .should_expand(true)
            .format(f)
        });

        if self.force_expand || has_ignore_ranges {
            format_indented.format(f)?;
        } else {
            best_fitting![format_inline, format_indented]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
        }

        Ok(())
    }
}

/// Format a list while preserving any ignore ranges as raw text.
fn format_list_with_ignored_ranges<'ast, T>(
    f: &mut Formatter<'_, DestackFormatContext<'ast>>,
    elements: &[LocalNodeId<T>],
    ignore_ranges: &HashMap<u32, Span>,
    separator: &'static str,
) -> FormatResult<bool>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T>,
{
    let mut skip_until: Option<u32> = None;
    let mut needs_separator = false;

    for element_id in elements {
        let element_span = f.context().get_span(*element_id);
        if let Some(skip_end) = skip_until {
            if element_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        if let Some(range_span) = ignore_ranges.get(&element_id.id) {
            let raw = ignored_span_source(f.context(), *range_span);
            let starts_with_separator = raw.trim_start().starts_with(separator);
            let ends_with_separator = raw_ends_with_separator(&raw, separator);

            if needs_separator && !starts_with_separator {
                write!(f, [token(separator), soft_line_break_or_space()])?;
            }

            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            needs_separator = !ends_with_separator;
            continue;
        }

        if needs_separator {
            write!(f, [token(separator), soft_line_break_or_space()])?;
        }

        write!(f, [*element_id])?;
        needs_separator = true;
    }

    Ok(needs_separator)
}

/// Check whether a raw range ends with the separator after trimming comments.
fn raw_ends_with_separator(raw: &str, separator: &str) -> bool {
    let trimmed = strip_trailing_comments(raw);
    trimmed.trim_end().ends_with(separator)
}

/// Strip trailing line and block comments from a raw string.
fn strip_trailing_comments(raw: &str) -> &str {
    let mut text = raw;
    loop {
        let trimmed = text.trim_end_matches(|ch: char| ch.is_whitespace());
        if trimmed.is_empty() {
            return trimmed;
        }

        if let Some(block_start) = trimmed
            .rfind("*/")
            .and_then(|block_end| trimmed[..block_end].rfind("/*"))
        {
            text = &trimmed[..block_start];
            continue;
        }

        let line_start = trimmed.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let line = &trimmed[line_start..];
        if let Some(idx) = line.find("//") {
            text = &trimmed[..line_start + idx];
            continue;
        }

        return trimmed;
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
        allow_trailing_separator: true,
        force_expand: false,
        kind: ListKind::FunctionParameters,
        group_id: None,
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

        let is_typescript = f.context().options.language_type.is_typescript();
        let parameter_is_static = is_typescript && parameter_is_static(f.context(), node_id);

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
                    if parameter_is_static {
                        write!(f, [space(), Keyword::Extends, space(), ty])?;
                    } else {
                        write!(f, [token(":"), space(), ty])?;
                    }
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
                    if parameter_is_static {
                        write!(f, [space(), Keyword::Extends, space(), ty])?;
                    } else {
                        write!(f, [token(":"), space(), ty])?;
                    }
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
                if let Some(ty) = ty {
                    if parameter_is_static {
                        write!(f, [space(), Keyword::Extends, space(), ty])?;
                    } else {
                        write!(f, [token(":"), space(), ty])?;
                    }
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
                if let Some(ty) = ty {
                    if parameter_is_static {
                        write!(f, [space(), Keyword::Extends, space(), ty])?;
                    } else {
                        write!(f, [token(":"), space(), ty])?;
                    }
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Return whether a parameter is declared in the static parameter list.
fn parameter_is_static(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(parameter_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration = context.tree.get(LocalNodeId::<Declaration>::new(parent_id));
    declaration
        .static_parameters()
        .is_some_and(|parameters| parameters.contains(&parameter_id))
}

/// Return whether an argument should emit its prefix annotations.
fn argument_should_emit_prefix_annotations(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_call_or_new(context, argument_id) {
        return true;
    }

    if !argument_is_first_in_call_or_new(context, argument_id) {
        return true;
    }

    if argument_has_non_blank_prefix_annotation(context, argument_id) {
        return true;
    }

    !argument_has_blank_prefix_annotation(context, argument_id)
}

/// Return whether an argument belongs to a call or new expression.
fn argument_is_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    matches!(expression, Expression::Call { .. } | Expression::New { .. })
}

/// Return whether an argument is the first in its call/new argument list.
fn argument_is_first_in_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    match expression {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments
            .first()
            .is_some_and(|first| *first == argument_id),
        _ => false,
    }
}

/// Return whether an argument has a non-blank prefix annotation.
fn argument_has_non_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(
        |annotation_id| match context.tree.get::<Annotation>(*annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Doc { position, .. }
            | Annotation::Comment { position, .. }
            | Annotation::Decorator { position, .. } => matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ),
        },
    )
}

/// Return whether an argument has a blank prefix annotation.
fn argument_has_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.tree.get::<Annotation>(*annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if argument_should_emit_prefix_annotations(f.context(), node_id) {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

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
            Argument::Labeled {
                modifiers,
                label,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // label
                write!(f, [label])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                write!(f, [token(":"), space(), value])?;
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
                label,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [token("...")])?;
                if let Some(label) = label {
                    write!(f, [label])?;
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                    write!(f, [token(":"), space(), value])?;
                } else {
                    write!(f, [value])?;
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
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
