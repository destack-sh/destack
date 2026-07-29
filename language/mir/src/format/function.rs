use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attribute, write_attributes};

use crate::{
    Attribute, AttributeIdentifier, FormatMirNode, Function, FunctionHeaderSpans,
    LifetimeParameter, Linkage, Local, LocalNodeId, MirFormatContext, MirFormatter, Mutability,
    Tree, write_comments_after, write_comments_before, write_inline_comment_after,
    write_node_leading_comments, write_node_leading_comments_after_separator,
};

impl<'a> FormatMirNode<'a, Function> for Function {
    fn format_node(
        &self,
        id: LocalNodeId<Function>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // function name
        let name = f.context().function_name(id).to_string();

        // attributes
        format_function_attributes(id, self, f)?;

        // imported function
        if self.linkage.is_import() {
            f.context_mut().current_lifetimes = self.lifetimes.clone();
            write!(f, [token("external"), space()])?;
            format_function_keyword(self, f)?;
            write!(
                f,
                [
                    space(),
                    copied_text(&name),
                    format_with(|f| format_lifetimes(&self.lifetimes, f))
                ]
            )?;

            // external parameters
            format_function_parameters(id, self, true, f)?;

            write!(f, [token(":"), space(), self.return_type])?;
            format_lifetime_where(&self.lifetimes, f)?;
            f.context_mut().current_lifetimes.clear();

            return Ok(());
        }

        // exported linkage prefix
        if self.linkage == Linkage::Export {
            write!(f, [token("export"), space()])?;
        }

        // local index map and current function
        {
            let context = f.context_mut();
            context.local_indices.clear();
            for (i, local_id) in self.locals().iter().enumerate() {
                context.local_indices.insert(*local_id, i);
            }
            context.current_function = Some(id);
            context.current_lifetimes = self.lifetimes.clone();
        }

        // function header
        format_function_keyword(self, f)?;
        write!(f, [space(), copied_text(&name)])?;
        format_lifetimes(&self.lifetimes, f)?;

        // parameters
        format_function_parameters(id, self, false, f)?;

        write!(f, [token(":"), space(), self.return_type])?;
        format_lifetime_where(&self.lifetimes, f)?;
        write!(f, [space(), token("{"), hard_line_break()])?;

        // function body
        format_function_body(self, f)?;

        f.context_mut().current_function = None;
        f.context_mut().current_lifetimes.clear();
        write!(f, [token("}")])
    }
}

/// Format declared outlives rows as one trailing where clause.
pub(super) fn format_lifetime_where<'a>(
    lifetimes: &[LifetimeParameter],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let mut first = true;
    for (slot, lifetime) in lifetimes.iter().enumerate() {
        for target in &lifetime.outlives {
            if first {
                write!(f, [space(), token("where"), space()])?;
                first = false;
            } else {
                write!(f, [token(","), space()])?;
            }
            let name = |slot: usize| {
                lifetimes
                    .get(slot)
                    .and_then(|parameter| parameter.name)
                    .map(|name| f.context().strings.get(name).to_string())
                    .unwrap_or_else(|| format!("'l{slot}"))
            };
            let left = name(slot);
            let right = name(target.0 as usize);
            write!(
                f,
                [copied_text(&left), token(":"), space(), copied_text(&right)]
            )?;
        }
    }

    Ok(())
}

/// Format the async and generator function modifiers.
fn format_function_keyword<'a>(
    function: &Function,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if function
        .coroutine
        .is_some_and(|coroutine| coroutine.is_async())
    {
        write!(f, [token("async"), space()])?;
    }

    write!(f, [token("function")])?;
    if function
        .coroutine
        .is_some_and(|coroutine| coroutine.is_generator())
    {
        write!(f, [token("*")])?;
    }

    Ok(())
}
/// Format a declaration lifetime header.
fn format_lifetimes<'a>(
    lifetimes: &[LifetimeParameter],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if lifetimes.is_empty() {
        return Ok(());
    }

    write!(f, [token("<")])?;
    for (index, lifetime) in lifetimes.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        let name = lifetime
            .name
            .map(|name| f.context().strings.get(name).to_string())
            .unwrap_or_else(|| format!("'l{index}"));
        write!(f, [copied_text(&name)])?;
    }
    write!(f, [token(">")])
}

/// Format function attributes and derived tables.
pub(super) fn format_function_attributes<'a>(
    id: LocalNodeId<Function>,
    function: &Function,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let attributes = tree.attributes(id);
    let keyword_span = tree.keyword_span(id);
    let attribute_spans = tree.attribute_spans(id);

    // explicit attributes
    let mut explicit_attribute_end = None;

    if !attributes.is_empty() {
        if attribute_spans.len() == attributes.len() {
            for (index, attribute) in attributes.iter().enumerate() {
                let attribute_span = attribute_spans[index];

                // comments between explicit attributes
                if let Some(previous_end) = explicit_attribute_end {
                    write_comments_before(tree, previous_end, attribute_span.start, f)?;
                }

                write_attribute(attribute, f)?;
                write!(f, [hard_line_break()])?;
                explicit_attribute_end = Some(attribute_span.end);
            }
        } else {
            write_attributes(attributes, f)?;
        }
    }

    if !has_attribute(attributes, "environment", f)
        && let Some(environment) = &function.environment
    {
        write!(
            f,
            [
                token("@"),
                token("environment"),
                token("("),
                environment,
                token(")"),
                hard_line_break()
            ]
        )?;
    }

    if !has_attribute(attributes, "binding", f)
        && let Some(binding) = &function.binding
    {
        write!(f, [binding.as_ref(), hard_line_break()])?;
    }

    // comments before the function head
    if let Some(previous_end) = explicit_attribute_end
        && let Some(keyword_span) = keyword_span
    {
        write_comments_before(tree, previous_end, keyword_span.start, f)?;
    }

    Ok(())
}

/// Return whether one explicit attribute list contains a named attribute.
fn has_attribute(attributes: &[Attribute], name: &str, f: &MirFormatter<'_, '_>) -> bool {
    attributes.iter().any(|attribute| {
        matches!(
            attribute.name,
            AttributeIdentifier::Identifier(identifier)
                if f.context().strings.get(identifier) == name
        )
    })
}

/// Format the body of one local function.
fn format_function_body<'a>(function: &Function, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    let locals = function.locals();
    let blocks = function.blocks();

    // locals
    if !locals.is_empty() {
        write!(
            f,
            [block_indent(&format_with(
                |f: &mut Formatter<'_, 'a, MirFormatContext<'a>>| {
                    for (local_index, local_id) in locals.iter().enumerate() {
                        let next_boundary = locals
                            .get(local_index + 1)
                            .and_then(|next_local_id| f.context().tree.get_span(*next_local_id))
                            .or_else(|| {
                                blocks
                                    .first()
                                    .and_then(|block_id| f.context().tree.get_span(*block_id))
                            })
                            .map(|span| span.start)
                            .unwrap_or(0);

                        format_local_declaration(*local_id, local_index, next_boundary, f)?;
                    }

                    Ok(())
                }
            ))]
        )?;
        write!(f, [empty_line()])?;
    }

    // blocks
    for (block_index, block_id) in blocks.iter().enumerate() {
        if block_index > 0 {
            write!(f, [empty_line()])?;
        }

        if block_index > 0 || !locals.is_empty() {
            write_node_leading_comments_after_separator(f.context().tree, *block_id, f)?;
        } else {
            write_node_leading_comments(f.context().tree, *block_id, f)?;
        }

        write!(f, [block_id, hard_line_break()])?;

        let block_span = f.context().tree.get_span(*block_id);

        if block_index + 1 == blocks.len()
            && let Some(block_span) = block_span
        {
            let next_boundary = f
                .context()
                .current_function
                .and_then(|function_id| f.context().tree.get_span(function_id))
                .map(|span| span.end)
                .unwrap_or(block_span.end);

            write_comments_after(f.context().tree, block_span.end, next_boundary, f)?;
        }
    }

    Ok(())
}

/// Format one local declaration line.
fn format_local_declaration<'a>(
    local_id: LocalNodeId<Local>,
    local_index: usize,
    next_boundary: u32,
    f: &mut Formatter<'_, 'a, MirFormatContext<'a>>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // leading comments
    write_node_leading_comments(tree, local_id, f)?;

    let local = tree.get(local_id);

    // local header
    write!(
        f,
        [
            token("local"),
            space(),
            copied_text(&format!("l{local_index}")),
            token(":"),
            space(),
            local.ty
        ]
    )?;

    // mutability
    if local.mutability == Mutability::Immutable {
        write!(f, [token(","), space(), token("readonly")])?;
    }

    // trailing comment
    if let Some(local_span) = tree.get_span(local_id) {
        write_inline_comment_after(tree, local_span.end, next_boundary, f)?;
    }
    write!(f, [hard_line_break()])?;

    Ok(())
}

/// Format one function parameter list.
fn format_function_parameters<'a>(
    id: LocalNodeId<Function>,
    function: &Function,
    is_import: bool,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let parameter_spans = tree.function_parameter_spans(id);
    let header_spans = tree.function_header_spans(id);

    // parameter comment mode
    let has_parameter_comments =
        has_parameter_comments(tree, header_spans, !tree.tokens().is_empty());

    // canonical single line form
    if !has_parameter_comments || parameter_spans.len() != function.parameters.len() {
        write!(f, [token("(")])?;
        for (i, param) in function.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, [token(","), space()])?;
            }

            if is_import {
                write!(f, [param.ty])?;
            } else {
                write!(f, [&param.value, token(":"), space(), param.ty])?;
            }
        }
        write!(f, [token(")")])?;
        return Ok(());
    }

    // comment preserving multiline form
    let Some(header_spans) = header_spans else {
        unreachable!("function header spans checked before formatting parameter comments");
    };
    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut Formatter<'_, 'a, MirFormatContext<'a>>| {
                let tree = f.context().tree;
                let mut previous_end = header_spans.open_paren.end;

                for (index, (parameter, span)) in function
                    .parameters
                    .iter()
                    .zip(parameter_spans.iter())
                    .enumerate()
                {
                    write_comments_before(tree, previous_end, span.span.start, f)?;

                    if is_import {
                        write!(f, [parameter.ty])?;
                    } else {
                        write!(f, [&parameter.value, token(":"), space(), parameter.ty])?;
                    }

                    let next_boundary = if let Some(next_span) = parameter_spans.get(index + 1) {
                        next_span.span.start
                    } else {
                        header_spans.close_paren.start
                    };

                    if index + 1 < parameter_spans.len() {
                        write!(f, [token(",")])?;
                    }

                    let wrote_inline_comment =
                        write_inline_comment_after(tree, span.span.end, next_boundary, f)?;

                    write!(f, [hard_line_break()])?;

                    previous_end = if wrote_inline_comment {
                        next_boundary
                    } else {
                        span.span.end
                    };
                }

                Ok(())
            }
        ))]
    )?;
    write!(f, [token(")")])?;

    Ok(())
}

/// Return whether one parameter list needs comment preserving formatting.
fn has_parameter_comments(
    tree: &Tree,
    header_spans: Option<&FunctionHeaderSpans>,
    has_tokens: bool,
) -> bool {
    let Some(header_spans) = header_spans else {
        return false;
    };

    has_tokens
        && !tree
            .comments_between(header_spans.open_paren.end, header_spans.close_paren.start)
            .is_empty()
}
