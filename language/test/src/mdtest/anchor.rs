use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use destack_core::StringPool;
use destack_parser::Parser;
use destack_source::{File, FileId, FileType, LanguageType, Uri};

/// One exact word token span in stripped fixture text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WordSpan {
    /// The byte start offset.
    start: usize,
    /// The byte end offset.
    end: usize,
}

impl WordSpan {
    /// Return whether this token fully contains one point offset.
    fn contains_offset(self, offset: usize) -> bool {
        self.start < offset && offset < self.end
    }

    /// Return whether this token overlaps one range.
    fn overlaps(self, range: &Range<usize>) -> bool {
        self.start < range.end && range.start < self.end
    }
}

/// Validate exact word-token anchoring for one query range marker set.
pub fn validate_query_range_markers(
    file_path: &str,
    text: &str,
    markers: &[(&str, Range<usize>)],
) -> Result<(), String> {
    let word_spans = collect_word_spans(file_path, text);

    for (name, span) in markers {
        if !marker_requires_exact_identifier_bounds(name) {
            continue;
        }

        let overlapping = word_spans
            .iter()
            .copied()
            .filter(|word| word.overlaps(span))
            .collect::<Vec<_>>();

        if overlapping.is_empty() {
            continue;
        }

        if overlapping.len() != 1 {
            return Err(format!(
                "range marker `{name}` in `{file_path}` overlaps multiple identifier tokens: `{}`",
                &text[span.start..span.end],
            ));
        }

        let word = overlapping[0];
        if span.start != word.start || span.end != word.end {
            return Err(format!(
                "range marker `{name}` in `{file_path}` must exactly cover word token `{}`",
                &text[word.start..word.end],
            ));
        }
    }

    Ok(())
}

/// Validate exact point anchoring for one LSP inline marker set.
pub fn validate_lsp_point_markers(
    file_path: &str,
    text: &str,
    markers: &[(&str, usize)],
) -> Result<(), String> {
    let word_spans = collect_word_spans(file_path, text);

    for (name, offset) in markers {
        let Some(word) = word_spans
            .iter()
            .copied()
            .find(|word| word.contains_offset(*offset))
        else {
            continue;
        };

        return Err(format!(
            "point marker `{name}` in `{file_path}` splits word token `{}`",
            &text[word.start..word.end],
        ));
    }

    Ok(())
}

/// Return whether one query marker name should be exact over identifiers.
fn marker_requires_exact_identifier_bounds(name: &str) -> bool {
    !matches!(name, "selection") && !name.starts_with("range:")
}

/// Collect exact word token spans for one stripped fixture file.
fn collect_word_spans(file_path: &str, text: &str) -> Vec<WordSpan> {
    let path = Path::new(file_path);
    let file_type = FileType::from_path_or_unknown(path);
    let language_type =
        LanguageType::try_from(file_type).expect("file type has no parser language");
    let uri = Uri::from_string(format!("/fixture/{file_path}"));
    let file = Arc::new(File::from_text(
        FileId(0),
        file_path.to_string(),
        uri,
        None,
        file_type,
        text.to_string(),
    ));
    let mut parser = Parser::lex_file(file, language_type, Arc::new(StringPool::new()));
    let (tokens, _) = parser.take_tokens();

    tokens
        .into_iter()
        .filter_map(|token| {
            let start = token.span.start as usize;
            let end = token.span.end as usize;
            let text = &text[start..end];
            let is_word = text
                .chars()
                .next()
                .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
                && text
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric());

            if is_word {
                Some(WordSpan { start, end })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{validate_lsp_point_markers, validate_query_range_markers};

    /// Query identifier ranges should cover the whole identifier.
    #[test]
    fn test_reject_query_partial_identifier_range() {
        let error = validate_query_range_markers(
            "main.ds",
            "const value = other;\n",
            &[("use:value", 6..10)],
        )
        .expect_err("expected strict anchor failure");

        assert!(error.contains("must exactly cover word token `value`"));
    }

    /// Query selection ranges may span arbitrary source.
    #[test]
    fn test_allow_query_selection_range() {
        validate_query_range_markers(
            "main.ds",
            "if (flag) { const a = 1; } const b = 2;\n",
            &[("selection", 3..31)],
        )
        .expect("selection markers should stay flexible");
    }

    /// LSP point markers should not split identifiers.
    #[test]
    fn test_reject_lsp_identifier_split_point() {
        let error = validate_lsp_point_markers("main.ds", "const value = 1;\n", &[("hover", 8)])
            .expect_err("expected strict anchor failure");

        assert!(error.contains("splits word token `value`"));
    }

    /// LSP point markers may sit inside string literals.
    #[test]
    fn test_allow_lsp_point_inside_string_literal() {
        validate_lsp_point_markers("main.ds", "const value = \"hello\";\n", &[("edit", 16)])
            .expect("string literal points should remain allowed");
    }
}
