use std::collections::HashMap;
use std::marker::PhantomData;

use destack_fir::format::{FormatResult, GroupId};
use destack_workspace::TrailingComma;

use crate::directive::{collect_ignore_ranges_for_nodes, ignored_span_source, write_ignored_span};
use crate::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::scan::{
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_annotation,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, CommentStyle, Declaration, Expression, FunctionKind, Keyword,
    LocalNodeId, Member, Node, NodeTree, NodeTreeImpl, NodeType, Parameter, Property, TokenType,
    TypeBinaryOperator,
};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
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
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Expression> + NodeTreeImpl<Declaration>,
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
    elements: &'e [LocalNodeId<T>],

    _phantom: PhantomData<&'ast ()>,
}

#[allow(dead_code)]
impl<'ast, 'e, T> ListLike<'ast, 'e, T>
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Expression> + NodeTreeImpl<Declaration>,
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
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Expression> + NodeTreeImpl<Declaration>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        let options = &f.context().options;
        let trailing_comma_option = options.trailing_comma;
        let has_elements = !self.elements.is_empty();

        // empty lists do not need any layout planning
        if !has_elements {
            f.context()
                .increment_counter("profile.list_like.empty.fast_path", 1);
            write!(f, [token(self.start_token), token(self.end_token)])?;
            return Ok(());
        }

        let should_add_trailing = has_elements
            && self.allow_trailing_separator
            && self.kind.should_add_trailing_comma(trailing_comma_option);
        let should_add_space = self.include_space && options.bracket_spacing && has_elements;

        let ignore_ranges_by_id = if f.context().has_ignore_directive_markers() {
            let comment_tokens = f.context().comment_tokens();
            let ignore_ranges_by_id =
                collect_ignore_ranges_for_nodes(f.context(), self.elements, comment_tokens);
            if ignore_ranges_by_id.is_empty() {
                None
            } else {
                Some(ignore_ranges_by_id)
            }
        } else {
            None
        };
        let has_ignore_ranges = ignore_ranges_by_id.is_some();
        let body = &format_with(|f| {
            // leading space
            if should_add_space {
                write!(f, [if_group_fits_on_line(&space())])?;
            }

            let mut needs_trailing_separator = true;
            if let Some(ignore_ranges_by_id) = ignore_ranges_by_id.as_ref() {
                needs_trailing_separator = format_list_with_ignored_ranges(
                    f,
                    self.elements,
                    ignore_ranges_by_id,
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
            if has_elements
                && self.force_trailing_separator
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

        // grouped lists can choose inline or expanded shape without best fitting
        let format_grouped = format_with(|f| {
            group(&format_args![
                &token(self.start_token),
                soft_block_indent(body),
                &token(self.end_token)
            ])
            .with_id(self.group_id)
            .should_expand(self.force_expand || has_ignore_ranges)
            .format(f)
        });

        // small annotation free lists almost always fit inline, skip best fitting probing
        let can_use_single_element_inline_fast_path = !self.force_expand
            && !has_ignore_ranges
            && self.group_id.is_some()
            && self.elements.len() == 1;
        if can_use_single_element_inline_fast_path {
            let element_id = self.elements[0];
            let element_span = f.context().get_span(element_id);
            let element_source_len = f.context().span_char_len(element_span);
            let compact_single_element_limit = usize::from(options.line_width).min(28);
            let inline_call_parent_fits = if self.start_token == "(" && self.end_token == ")" {
                f.context()
                    .get_parent(element_id)
                    .is_some_and(|(parent_id, parent_type)| {
                        if parent_type != NodeType::Expression {
                            return false;
                        }

                        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                        let parent_span = f.context().get_span::<Expression>(parent_expression_id);
                        let parent_len = f.context().span_char_len(parent_span);
                        parent_len <= usize::from(options.line_width)
                    })
            } else {
                true
            };
            let can_keep_single_element_inline = !f.context().has_newline(element_span)
                && !f.context().has_annotation(element_id)
                && inline_call_parent_fits
                && element_source_len <= compact_single_element_limit;
            if can_keep_single_element_inline {
                f.context()
                    .increment_counter("profile.list_like.single_inline.fast_path", 1);
                format_inline.format(f)?;
                return Ok(());
            }
        }

        // grouped lists use one adaptive layout path
        if self.group_id.is_some() {
            format_grouped.format(f)?;
            return Ok(());
        }

        if self.force_expand || has_ignore_ranges {
            format_indented.format(f)?;
        } else if self.group_id.is_none() {
            let no_group_counter = match (self.start_token, self.end_token) {
                ("(", ")") => "profile.list_like.no_group.paren",
                ("[", "]") => "profile.list_like.no_group.bracket",
                ("{", "}") => "profile.list_like.no_group.brace",
                ("<", ">") => "profile.list_like.no_group.angle",
                _ => "profile.list_like.no_group.other",
            };
            f.context().increment_counter(no_group_counter, 1);
            if self.start_token == "<" && self.end_token == ">" {
                if self.elements.len() == 1 {
                    let element_id = self.elements[0];
                    let preserve_source_breaks =
                        parent_expression_has_linebreak_around_element(f.context(), element_id);
                    if preserve_source_breaks {
                        format_grouped.format(f)?;
                    } else {
                        group(&format_args![
                            &token(self.start_token),
                            body,
                            &token(self.end_token)
                        ])
                        .format(f)?;
                    }
                } else {
                    format_grouped.format(f)?;
                }
            } else {
                format_grouped.format(f)?;
            }
        } else {
            format_grouped.format(f)?;
        }

        Ok(())
    }
}

/// Return whether the parent expression includes source line breaks around this element.
fn parent_expression_has_linebreak_around_element<'ast, T>(
    context: &DestackFormatContext<'ast>,
    element_id: LocalNodeId<T>,
) -> bool
where
    T: Node + Clone + FormatNode<'ast, T>,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Expression>,
{
    let Some((parent_id, parent_type)) = context.get_parent(element_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_span = context.get_span(parent_expression_id);
    let element_span = context.get_span(element_id);
    if parent_span.file != element_span.file {
        return false;
    }

    let has_leading_break = parent_span.start < element_span.start
        && context.has_newline(Span::new(
            parent_span.file,
            parent_span.start,
            element_span.start,
        ));
    let has_trailing_break = element_span.end < parent_span.end
        && context.has_newline(Span::new(
            parent_span.file,
            element_span.end,
            parent_span.end,
        ));

    has_leading_break || has_trailing_break
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
    let mut text = raw.trim_end_matches(|ch: char| ch.is_whitespace());
    loop {
        if text.is_empty() {
            return text;
        }

        // strip trailing block comments only when they are truly at the end
        if text.ends_with("*/")
            && let Some(block_start) = text[..text.len().saturating_sub(2)].rfind("/*")
        {
            text = text[..block_start].trim_end_matches(|ch: char| ch.is_whitespace());
            continue;
        }

        // strip trailing line comments from the last line
        let line_start = text.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let line = &text[line_start..];
        if let Some(idx) = line.find("//") {
            text = text[..line_start + idx].trim_end_matches(|ch: char| ch.is_whitespace());
            continue;
        }

        return text;
    }
}

/// Format a list-like group for `elements`.
/// Begin with `start_token`.
/// End with `end_token`.
/// Separate entries with `separator`.
///
/// By default, treats the list as function params for trailing comma purposes.
/// Call `.as_collection()` for arrays, objects, and tuples.
pub(crate) fn list_like<'ast, 'e, T>(
    start_token: &'static str,
    end_token: &'static str,
    separator: &'static str,
    elements: &'e [LocalNodeId<T>],
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

/// Write one parameter type annotation with optional static parameter syntax.
fn write_parameter_type_with_infix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    parameter_is_static: bool,
    ty: Option<LocalNodeId<Expression>>,
) -> FormatResult<bool> {
    let Some(ty) = ty else {
        return Ok(false);
    };

    write!(f, [f.context().block_infix_annotations(node_id)])?;
    if parameter_is_static {
        write!(f, [space(), Keyword::Extends, space(), ty])?;
    } else {
        let has_infix_annotations = f.context().has_infix_annotation(node_id);
        if has_infix_annotations {
            write!(f, [space(), token(":"), space(), ty])?;
        } else {
            write!(f, [token(":"), space(), ty])?;
        }
    }
    Ok(true)
}

/// Return whether all prefix annotations on one variadic parameter follow `...`.
fn parameter_prefix_annotations_follow_spread(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let Some(annotations) = context.get_annotations(parameter_id) else {
        return false;
    };

    let mut has_prefix_annotation = false;
    for annotation_id in annotations {
        let position = context.get_annotation(annotation_id).position();
        if !matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            continue;
        }

        has_prefix_annotation = true;
        let previous_token =
            previous_non_whitespace_token_before_annotation(context, annotation_id);
        if !previous_token.is_some_and(|token| token.token.ty == TokenType::Spread) {
            return false;
        }
    }

    has_prefix_annotation
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<Parameter>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let is_variadic_parameter = matches!(
            self,
            Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
        );
        let defer_prefix_annotations_after_spread = is_variadic_parameter
            && parameter_prefix_annotations_follow_spread(f.context(), node_id);
        if !defer_prefix_annotations_after_spread {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

        let is_typescript = f.context().options.language_type.is_typescript();
        let parameter_is_static = is_typescript && parameter_is_static(f.context(), node_id);
        let wrote_type_infix = match self {
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
                let wrote_type_infix =
                    write_parameter_type_with_infix(f, node_id, parameter_is_static, *ty)?;
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
                wrote_type_infix
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
                let wrote_type_infix =
                    write_parameter_type_with_infix(f, node_id, parameter_is_static, *ty)?;
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
                wrote_type_infix
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
                // spread seam prefix annotations
                if defer_prefix_annotations_after_spread {
                    write!(f, [f.context().any_prefix_annotations(node_id)])?;
                }
                // name
                write!(f, [name])?;
                // type
                write_parameter_type_with_infix(f, node_id, parameter_is_static, *ty)?
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
                // spread seam prefix annotations
                if defer_prefix_annotations_after_spread {
                    write!(f, [f.context().any_prefix_annotations(node_id)])?;
                }
                // pattern
                write!(f, [pattern])?;
                // type
                write_parameter_type_with_infix(f, node_id, parameter_is_static, *ty)?
            }
        };

        if wrote_type_infix {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else {
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        }

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
    match parent_type {
        NodeType::Declaration => {
            let declaration = context.tree.get(LocalNodeId::<Declaration>::new(parent_id));
            declaration
                .static_parameters()
                .is_some_and(|parameters| parameters.contains(&parameter_id))
        }
        NodeType::Property => {
            let property = context.tree.get(LocalNodeId::<Property>::new(parent_id));
            let Property::Method { signature, .. } = property else {
                return false;
            };

            signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .is_some_and(|parameters| parameters.contains(&parameter_id))
        }
        NodeType::Member => {
            let member = context.tree.get(LocalNodeId::<Member>::new(parent_id));
            match member {
                Member::Type {
                    static_parameters, ..
                } => static_parameters
                    .as_ref()
                    .is_some_and(|parameters| parameters.contains(&parameter_id)),
                Member::Method { signature, .. } => signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    .is_some_and(|parameters| parameters.contains(&parameter_id)),
                Member::Field { .. }
                | Member::ComptimeConst { .. }
                | Member::Embed { .. }
                | Member::StaticBlock { .. }
                | Member::ComptimeBlock { .. } => false,
            }
        }
        _ => false,
    }
}

/// Return whether an argument should emit its prefix annotations.
fn argument_should_emit_prefix_annotations(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if argument_has_satisfies_static_seam_prefix_line_comment(context, argument_id) {
        return false;
    }

    if !argument_is_call_or_new(context, argument_id) {
        return true;
    }

    if argument_has_non_blank_prefix_annotation(context, argument_id) {
        return true;
    }

    if argument_has_blank_prefix_annotation_before_separator(context, argument_id) {
        return false;
    }

    if !argument_is_first_in_call_or_new(context, argument_id) {
        return true;
    }

    !argument_has_blank_prefix_annotation(context, argument_id)
}

/// Return one satisfies static seam line comment source for this argument when present.
pub(super) fn argument_satisfies_static_seam_comment_source(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<String> {
    let seam_comment_id = argument_satisfies_static_seam_comment_id(context, argument_id)?;
    let span = context.get_annotation_span(seam_comment_id);
    Some(context.get_span_str(span).trim().to_string())
}

/// Return one satisfies static seam line comment annotation id for this argument when present.
pub(super) fn argument_satisfies_static_seam_comment_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Annotation>> {
    if !argument_is_first_static_argument_of_satisfies_right_path(context, argument_id) {
        return None;
    }

    let Some(annotations) = context.get_annotations(argument_id) else {
        return None;
    };

    let mut seam_comment_id = None;
    for annotation_id in annotations {
        let Annotation::Comment {
            node,
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
        } = context.get_annotation(annotation_id)
        else {
            continue;
        };

        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != CommentStyle::Slash {
            continue;
        }

        seam_comment_id = Some(annotation_id);
        break;
    }

    seam_comment_id
}

/// Return whether one argument is the first static argument in a `satisfies` rhs path with multiple arguments.
fn argument_is_first_static_argument_of_satisfies_right_path(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_first_static_argument_of_multi_argument_path(context, argument_id) {
        return false;
    }

    let Some((path_expression_id, path_parent_type)) = context.get_parent(argument_id) else {
        return false;
    };
    if path_parent_type != NodeType::Expression {
        return false;
    }

    let path_expression_id = LocalNodeId::<Expression>::new(path_expression_id);
    let Some((type_binary_id, type_binary_parent_type)) = context.get_parent(path_expression_id)
    else {
        return false;
    };
    if type_binary_parent_type != NodeType::Expression {
        return false;
    }

    let type_binary_id = LocalNodeId::<Expression>::new(type_binary_id);
    let Expression::TypeBinary {
        operator: TypeBinaryOperator::Satisfies,
        right,
        ..
    } = context.tree.get(type_binary_id)
    else {
        return false;
    };

    let right_expression_id = match context.tree.get(*right) {
        Expression::Parenthesized { expression } | Expression::Statement(expression) => *expression,
        _ => *right,
    };

    right_expression_id == path_expression_id
}

/// Return whether one argument is the first static argument of a multi-argument path static list.
fn argument_is_first_static_argument_of_multi_argument_path(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(parent_id);
    let static_arguments = match context.tree.get(expression_id) {
        // keep seam remapping constrained to the formatter path we re-render explicitly
        Expression::Path {
            path,
            static_arguments,
        } if path.segments.len() == 1 => static_arguments.as_ref(),
        _ => None,
    };

    let Some(static_arguments) = static_arguments else {
        return false;
    };
    if static_arguments.len() <= 1 {
        return false;
    }

    static_arguments
        .first()
        .is_some_and(|first| *first == argument_id)
}

/// Return whether this argument has one satisfies static seam prefix line comment.
fn argument_has_satisfies_static_seam_prefix_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_satisfies_static_seam_comment_id(context, argument_id).is_some()
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
        |annotation_id| match context.get_annotation(*annotation_id) {
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
            context.get_annotation(*annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether an argument has a blank prefix annotation before a separator.
fn argument_has_blank_prefix_annotation_before_separator(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Blank {
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            ..
        } = context.get_annotation(*annotation_id)
        else {
            return false;
        };

        next_non_whitespace_token_after_annotation(context, *annotation_id)
            .is_some_and(|token| token.token.ty == TokenType::Comma)
    })
}

/// Return whether a lambda argument has an inline prefix comment that must break.
fn argument_prefix_lambda_comment_needs_forced_break(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.get_annotation(*annotation_id) else {
            return false;
        };
        if position != AnnotationPosition::BlockPrefix {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let previous_token =
            previous_non_whitespace_token_before_annotation(context, *annotation_id);
        let next_token = next_non_whitespace_token_after_annotation(context, *annotation_id);
        previous_token.is_some_and(|token| {
            matches!(
                token.token.ty,
                TokenType::OpenParenthesis
                    | TokenType::OpenBracket
                    | TokenType::OpenBrace
                    | TokenType::LessThan
            )
        }) && next_token.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
    })
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // fast path: annotation free positional and spread arguments dominate call sites
        // and don't need the expensive prefix and trailing annotation checks
        let has_argument_annotation = f.context().has_annotation(node_id);
        if !has_argument_annotation {
            match self {
                Argument::Named {
                    modifiers: None,
                    name,
                    value,
                } => {
                    write!(f, [*name, token(":"), space(), *value])?;
                    return Ok(());
                }
                Argument::Labeled {
                    modifiers: None,
                    label,
                    value,
                } => {
                    write!(f, [*label, token(":"), space(), *value])?;
                    return Ok(());
                }
                Argument::Positional {
                    modifiers: None,
                    value,
                } => {
                    write!(f, [*value])?;
                    return Ok(());
                }
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                } => {
                    write!(f, [token("..."), *value])?;
                    return Ok(());
                }
                Argument::Spread {
                    modifiers: None,
                    label: Some(label),
                    value,
                } => {
                    write!(f, [token("..."), *label, token(":"), space(), *value])?;
                    return Ok(());
                }
                _ => {}
            }
        }

        // lambda argument comments are handled at the lambda arrow site
        let should_emit_prefix_annotations =
            argument_should_emit_prefix_annotations(f.context(), node_id);
        let should_preserve_blank_line_before_prefix_comment =
            argument_prefix_comment_has_leading_blank_line_after_separator(f.context(), node_id);
        let has_argument_prefix_comment =
            argument_has_prefix_comment_annotation(f.context(), node_id);
        let has_lambda_value = argument_contains_lambda_value(f.context(), self);
        let force_break_after_lambda_prefix_comment = has_lambda_value
            && argument_prefix_lambda_comment_needs_forced_break(f.context(), node_id);

        if has_lambda_value {
            write!(f, [f.context().block_prefix_annotations(node_id)])?;
        } else if should_emit_prefix_annotations {
            if should_preserve_blank_line_before_prefix_comment && has_argument_prefix_comment {
                write!(f, [hard_line_break()])?;
            }
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
                if should_preserve_blank_line_before_prefix_comment && !has_argument_prefix_comment
                {
                    write!(f, [hard_line_break()])?;
                }
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
                if should_preserve_blank_line_before_prefix_comment && !has_argument_prefix_comment
                {
                    write!(f, [hard_line_break()])?;
                }
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { modifiers, value } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // value
                if force_break_after_lambda_prefix_comment {
                    write!(f, [hard_line_break()])?;
                }
                if should_preserve_blank_line_before_prefix_comment && !has_argument_prefix_comment
                {
                    write!(f, [hard_line_break()])?;
                }
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
                    if should_preserve_blank_line_before_prefix_comment
                        && !has_argument_prefix_comment
                    {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [token(":"), space(), value])?;
                } else {
                    if force_break_after_lambda_prefix_comment {
                        write!(f, [hard_line_break()])?;
                    }
                    if should_preserve_blank_line_before_prefix_comment
                        && !has_argument_prefix_comment
                    {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [value])?;
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Return whether the argument itself has a prefix comment annotation.
fn argument_has_prefix_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.get_annotation(*annotation_id),
            Annotation::Comment {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether an argument has a prefix comment separated by a blank line after a comma.
fn argument_prefix_comment_has_leading_blank_line_after_separator(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_call_or_new(context, argument_id) {
        return false;
    }
    let Some(prefix_comment_start) =
        first_prefix_comment_annotation_start_for_argument(context, argument_id)
    else {
        return false;
    };
    let Some(previous_argument_id) = previous_dynamic_argument_in_call_or_new(context, argument_id)
    else {
        return false;
    };

    let previous_span = context.get_span(previous_argument_id);
    if previous_span.end >= prefix_comment_start {
        return false;
    }

    let between = context.get_span_str(Span::new(
        previous_span.file,
        previous_span.end,
        prefix_comment_start,
    ));
    between
        .chars()
        .filter(|character| *character == '\n')
        .count()
        >= 2
}

/// Return the start offset of the first prefix comment on an argument or its value.
fn first_prefix_comment_annotation_start_for_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<u32> {
    let argument_annotation_start = context
        .get_annotations(argument_id)
        .and_then(|annotations| {
            annotations.iter().find_map(|annotation_id| {
                let annotation = context.get_annotation(*annotation_id);
                let is_prefix_comment = matches!(
                    annotation,
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                );
                if is_prefix_comment {
                    Some(context.get_annotation_span(*annotation_id).start)
                } else {
                    None
                }
            })
        });

    let argument = context.tree.get(argument_id);
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    let value_annotation_start = context.get_annotations(value_id).and_then(|annotations| {
        annotations.iter().find_map(|annotation_id| {
            let annotation = context.get_annotation(*annotation_id);
            let is_prefix_comment = matches!(
                annotation,
                Annotation::Comment {
                    position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                    ..
                }
            );
            if is_prefix_comment {
                Some(context.get_annotation_span(*annotation_id).start)
            } else {
                None
            }
        })
    });

    match (argument_annotation_start, value_annotation_start) {
        (Some(argument_start), Some(value_start)) => Some(argument_start.min(value_start)),
        (Some(argument_start), None) => Some(argument_start),
        (None, Some(value_start)) => Some(value_start),
        (None, None) => None,
    }
}

/// Return the previous dynamic argument in a call or new expression.
fn previous_dynamic_argument_in_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Argument>> {
    let (parent_id, parent_type) = context.get_parent(argument_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    let arguments = match expression {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments,
        _ => return None,
    };

    let index = arguments
        .iter()
        .position(|argument| *argument == argument_id)?;
    index
        .checked_sub(1)
        .and_then(|index| arguments.get(index).copied())
}

/// Return whether this argument wraps a lambda declaration expression.
fn argument_contains_lambda_value(context: &DestackFormatContext<'_>, argument: &Argument) -> bool {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

#[cfg(test)]
mod tests {
    use super::{raw_ends_with_separator, strip_trailing_comments};
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
    fn test_format_parameter_comment_between_name_and_type() {
        assert_format!(
            "x /* a */ : number",
            "/* a */ x: number",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_optional_parameter_comment_between_name_and_type() {
        assert_format!(
            "x? /* a */ : number",
            "/* a */ x?: number",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_comment_before_name() {
        assert_format!(
            "/* a */ x: number",
            "/* a */ x: number",
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

    #[test]
    fn test_strip_trailing_comments_keeps_leading_ignore_block_comment() {
        let raw = "/* biome-ignore format: keep */\nsomeProperty:    alias,";
        let stripped = strip_trailing_comments(raw);

        assert_eq!(stripped, raw);
    }

    #[test]
    fn test_raw_ends_with_separator_for_ignored_field_with_leading_comment() {
        let raw = "/* biome-ignore format: keep */\nsomeProperty:    alias,";
        assert!(raw_ends_with_separator(raw, ","));
    }

    #[test]
    fn test_raw_ends_with_separator_with_trailing_line_comment() {
        let raw = "someProperty: alias, // keep";
        assert!(raw_ends_with_separator(raw, ","));
    }
}
