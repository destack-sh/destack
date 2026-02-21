use std::collections::HashMap;
use std::marker::PhantomData;

use destack_fir::format::{FormatResult, GroupId};
use destack_workspace::TrailingComma;

use crate::analysis::scan::{first_non_trivia_token_in_span, last_non_trivia_token_in_span};
use crate::directive::{collect_ignore_ranges_for_nodes, write_ignored_span};
use crate::{DestackFormatContext, FormatNode};
use destack_ast::{
    Declaration, Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType, TokenType,
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
                .increment_counter("profile.list_like.empty.short_circuit", 1);
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
    let Some((parent_id, parent_type)) = context.parent(element_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_span = context.span(parent_expression_id);
    let element_span = context.span(element_id);
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
        let element_span = f.context().span(*element_id);
        if let Some(skip_end) = skip_until {
            if element_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        if let Some(range_span) = ignore_ranges.get(&element_id.id) {
            let starts_with_separator =
                ignored_range_starts_with_separator(f.context(), *range_span, separator);
            let ends_with_separator =
                ignored_range_ends_with_separator(f.context(), *range_span, separator);

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

/// Return the token type used for one list separator string.
fn separator_token_type(separator: &str) -> Option<TokenType> {
    match separator {
        "," => Some(TokenType::Comma),
        ";" => Some(TokenType::Semicolon),
        ":" => Some(TokenType::Colon),
        "|" => Some(TokenType::ElementwiseOr),
        "&" => Some(TokenType::ElementwiseAnd),
        _ => None,
    }
}

/// Return whether one ignored range starts with a separator token.
fn ignored_range_starts_with_separator(
    context: &DestackFormatContext<'_>,
    range_span: Span,
    separator: &str,
) -> bool {
    let Some(separator_token) = separator_token_type(separator) else {
        return false;
    };

    first_non_trivia_token_in_span(context, range_span)
        .is_some_and(|token| token.token.ty == separator_token)
}

/// Return whether one ignored range ends with a separator token.
fn ignored_range_ends_with_separator(
    context: &DestackFormatContext<'_>,
    range_span: Span,
    separator: &str,
) -> bool {
    let Some(separator_token) = separator_token_type(separator) else {
        return false;
    };

    last_non_trivia_token_in_span(context, range_span)
        .is_some_and(|token| token.token.ty == separator_token)
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
