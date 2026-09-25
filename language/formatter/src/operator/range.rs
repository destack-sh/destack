use crate::annotation::FormatTrailingComments;
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{Comment, RangeEnd, TokenType};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{space, token};
use tspp_fir::write;
use tspp_source::{FileId, Span};

/// Write one compact range operator with readable comment boundaries.
pub(crate) fn write_range_operator(
    f: &mut TsppFormatter<'_, '_>,
    range_span: Span,
    start_end: Option<u32>,
    end_start: Option<u32>,
    end_kind: RangeEnd,
) -> FormatResult<()> {
    let token_type = range_token_type(end_kind);
    let token_value = range_token_value(end_kind);
    let operator_span =
        range_operator_span(f.context(), range_span, token_type, start_end, end_start);
    let left_comments = operator_span.and_then(|operator_span| {
        range_operator_left_comments(f.context(), operator_span, start_end)
    });
    let right_comments = operator_span.and_then(|operator_span| {
        range_operator_right_comments(f.context(), operator_span, end_start)
    });
    let needs_spacing = operator_span.is_some_and(|operator_span| {
        range_operator_has_comment_boundary(
            f.context(),
            operator_span,
            start_end,
            end_start,
            right_comments.as_deref(),
        )
    });

    if let Some(left_comments) = &left_comments {
        write!(f, [FormatTrailingComments::Comments(left_comments)])?;
    }

    if needs_spacing && start_end.is_some() {
        write!(f, [space()])?;
    }

    write!(f, [token(token_value)])?;

    if let Some(right_comments) = &right_comments {
        write!(f, [FormatTrailingComments::Comments(right_comments)])?;
    }

    if needs_spacing && end_start.is_some() {
        write!(f, [space()])?;
    }

    Ok(())
}

/// Return the token type for one range operator.
fn range_token_type(end_kind: RangeEnd) -> TokenType {
    match end_kind {
        RangeEnd::Open => TokenType::Range,
        RangeEnd::Inclusive => TokenType::RangeInclusive,
    }
}

/// Return the printed text for one range operator.
fn range_token_value(end_kind: RangeEnd) -> &'static str {
    match end_kind {
        RangeEnd::Open => "..",
        RangeEnd::Inclusive => "..=",
    }
}

/// Return the concrete range operator token span.
fn range_operator_span(
    context: &TsppFormatContext<'_>,
    range_span: Span,
    token_type: TokenType,
    start_end: Option<u32>,
    end_start: Option<u32>,
) -> Option<Span> {
    let start = start_end.unwrap_or(range_span.start);
    let end = end_start.unwrap_or(range_span.end);
    let search_span = Span::new(range_span.file, start, end);

    context
        .tokens_in_span(search_span)
        .iter()
        .copied()
        .find(|token| token.token.ty() == token_type)
        .map(|token| token.span)
}

/// Return whether comments touch one range operator boundary.
fn range_operator_has_comment_boundary(
    context: &TsppFormatContext<'_>,
    operator_span: Span,
    start_end: Option<u32>,
    end_start: Option<u32>,
    right_comments: Option<&[Comment]>,
) -> bool {
    let has_left_comment = start_end.is_some_and(|start_end| {
        source_has_comment_in_range(context, operator_span.file, start_end, operator_span.start)
    });
    let has_right_comment = if let Some(end_start) = end_start {
        source_has_comment_in_range(context, operator_span.file, operator_span.end, end_start)
    } else {
        right_comments.is_some_and(|comments| !comments.is_empty())
    };

    has_left_comment || has_right_comment
}

/// Return whether source comments exist between two offsets.
fn source_has_comment_in_range(
    context: &TsppFormatContext<'_>,
    file: FileId,
    start: u32,
    end: u32,
) -> bool {
    if end <= start {
        return false;
    }

    let span = Span::new(file, start, end);

    !context.source_comments_intersecting_span(span).is_empty()
}

/// Return comments between the start bound and range operator.
fn range_operator_left_comments(
    context: &TsppFormatContext<'_>,
    operator_span: Span,
    start_end: Option<u32>,
) -> Option<Vec<Comment>> {
    let start_end = start_end?;
    let comments = context
        .comments()
        .comments_in_range(start_end, operator_span.start)
        .to_vec();

    (!comments.is_empty()).then_some(comments)
}

/// Return comments between the range operator and end bound.
fn range_operator_right_comments(
    context: &TsppFormatContext<'_>,
    operator_span: Span,
    end_start: Option<u32>,
) -> Option<Vec<Comment>> {
    let comments = if let Some(end_start) = end_start {
        context
            .comments()
            .comments_in_range(operator_span.end, end_start)
            .to_vec()
    } else {
        range_operator_dangling_comments(context, operator_span)
    };

    (!comments.is_empty()).then_some(comments)
}

/// Return block comments immediately after a range operator with no end bound.
fn range_operator_dangling_comments(
    context: &TsppFormatContext<'_>,
    operator_span: Span,
) -> Vec<Comment> {
    let source = context.source_text();
    let mut comments = Vec::new();
    let mut start = operator_span.end;

    for comment in context.comments().comments_after(operator_span.end) {
        if !comment.is_block() {
            break;
        }

        if !source.all_bytes_match(start, comment.span.start, |byte| byte.is_ascii_whitespace()) {
            break;
        }

        comments.push(*comment);
        start = comment.span.end;
    }

    comments
}
