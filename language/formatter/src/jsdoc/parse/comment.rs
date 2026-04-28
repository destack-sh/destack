use std::cell::OnceCell;

use destack_source::Span;

use super::model::JsdocText;
use super::scan::parse_jsdoc;
use super::tag::JsdocTag;

type ParsedJsdoc<'a> = (JsdocText<'a>, Vec<JsdocTag<'a>>);

#[derive(Debug, Clone)]
pub(in crate::jsdoc) struct Jsdoc<'a> {
    raw: &'a str,
    /// The cached parsed Jsdoc comment and tags.
    cached: OnceCell<ParsedJsdoc<'a>>,
    /// The source span of the Jsdoc comment body.
    pub(in crate::jsdoc) span: Span,
}

impl<'a> Jsdoc<'a> {
    /// Create a Jsdoc parser for the inner comment body.
    pub(in crate::jsdoc) fn new(comment_content: &'a str, span: Span) -> Jsdoc<'a> {
        Self {
            raw: comment_content,
            cached: OnceCell::new(),
            span,
        }
    }

    /// Return the leading description.
    pub(in crate::jsdoc) fn comment(&self) -> JsdocText<'a> {
        self.parse().0
    }

    /// Return all parsed tags.
    pub(in crate::jsdoc) fn tags(&self) -> &[JsdocTag<'a>] {
        &self.parse().1
    }

    fn parse(&self) -> &ParsedJsdoc<'a> {
        self.cached.get_or_init(|| parse_jsdoc(self.raw, self.span))
    }
}

#[cfg(test)]
mod test {
    use destack_source::{FileId, Span};

    #[test]
    fn parses_with_double_backticks() {
        let source = "\
 * Handle whitespace rendering based on meta string
 *
 * `` ```js :whitespace[=all|boundary|trailing] ``
 *
 * @param parser - Code parser instance
 * @param meta - Meta string
 * @param globalOption - Global whitespace option
 *
 * @example
 * ```ts
 * metaWhitespace(parser, ':whitespace=all', true)
 * ```
 ";
        #[expect(clippy::cast_possible_truncation)]
        let jsdoc = super::Jsdoc::new(source, Span::new(FileId::EPHEMERAL, 0, source.len() as u32));
        let tags = jsdoc.tags();
        assert_eq!(tags.len(), 4);
        assert_eq!(tags[0].kind.parsed(), "param");
        assert_eq!(tags[1].kind.parsed(), "param");
        assert_eq!(tags[2].kind.parsed(), "param");
        assert_eq!(tags[3].kind.parsed(), "example");
    }

    #[test]
    fn parses_tags_after_math_interval_notation() {
        let source = "\
 * Random float in [min, max).
 * @param {number} min - Minimum float value.
 * @param {number} max - Maximum float value.
 * @returns {number} Random float in [min, max).
 ";
        #[expect(clippy::cast_possible_truncation)]
        let jsdoc = super::Jsdoc::new(source, Span::new(FileId::EPHEMERAL, 0, source.len() as u32));
        let tags = jsdoc.tags();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].kind.parsed(), "param");
        assert_eq!(tags[1].kind.parsed(), "param");
        assert_eq!(tags[2].kind.parsed(), "returns");
    }
}
