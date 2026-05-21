use destack_source::Span;

use super::model::JsdocText;
use super::scan::parse_jsdoc;
use super::tag::JsdocTag;

type ParsedJsdoc<'a> = (JsdocText<'a>, Vec<JsdocTag<'a>>);

#[derive(Debug, Clone)]
pub(in crate::jsdoc) struct Jsdoc<'a> {
    raw: &'a str,
    /// The source span of the Jsdoc comment body.
    pub(in crate::jsdoc) span: Span,
}

impl<'a> Jsdoc<'a> {
    /// Create a Jsdoc parser for the inner comment body.
    pub(in crate::jsdoc) fn new(comment_content: &'a str, span: Span) -> Jsdoc<'a> {
        Self {
            raw: comment_content,
            span,
        }
    }

    /// Parse this Jsdoc body into its description and tags.
    pub(in crate::jsdoc) fn parse(&self) -> ParsedJsdoc<'a> {
        parse_jsdoc(self.raw, self.span)
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
        let file_id = FileId::from_source_bytes(source.as_bytes());
        #[expect(clippy::cast_possible_truncation)]
        let jsdoc = super::Jsdoc::new(source, Span::new(file_id, 0, source.len() as u32));
        let (_, tags) = jsdoc.parse();

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
        let file_id = FileId::from_source_bytes(source.as_bytes());
        #[expect(clippy::cast_possible_truncation)]
        let jsdoc = super::Jsdoc::new(source, Span::new(file_id, 0, source.len() as u32));
        let (_, tags) = jsdoc.parse();

        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].kind.parsed(), "param");
        assert_eq!(tags[1].kind.parsed(), "param");
        assert_eq!(tags[2].kind.parsed(), "returns");
    }
}
