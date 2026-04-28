use destack_source::Span;

use super::model::{JsdocTagKind, JsdocTagName, JsdocTagType, JsdocText};
use super::range;

/// A parsed Jsdoc tag with lazily interpreted body fields.
#[derive(Debug, Clone)]
pub(in crate::jsdoc) struct JsdocTag<'a> {
    /// The tag kind, including the leading `@`.
    pub(in crate::jsdoc) kind: JsdocTagKind<'a>,
    /// The raw tag body after the kind.
    body_raw: &'a str,
    /// The span of the tag body.
    body_span: Span,
}

impl<'a> JsdocTag<'a> {
    pub(in crate::jsdoc) fn new(
        kind: JsdocTagKind<'a>,
        body_raw: &'a str,
        body_span: Span,
    ) -> JsdocTag<'a> {
        Self {
            kind,
            body_raw,
            body_span,
        }
    }

    /// Use for simple tags like `@access` and `@deprecated`.
    /// Comment can be multiline.
    ///
    /// Variants:
    /// ```text
    /// @kind comment
    /// @kind
    /// ```
    pub(in crate::jsdoc) fn comment(&self) -> JsdocText<'a> {
        JsdocText::new(self.body_raw, self.body_span)
    }

    /// Use for tags like `@yields` and `@returns`.
    /// Comment can be multiline.
    ///
    /// Variants:
    /// ```text
    /// @kind {type} comment
    /// @kind {type}
    /// @kind comment
    /// @kind
    /// ```
    pub(in crate::jsdoc) fn type_comment(&self) -> (Option<JsdocTagType<'a>>, JsdocText<'a>) {
        let (tag_type, description) = match range::find_type_range(self.body_raw) {
            Some((type_start, type_end)) => {
                // include whitespace for comment trimming
                let description_start = type_end;
                (
                    Some(JsdocTagType::new(
                        &self.body_raw[type_start..type_end],
                        self.body_span.subspan(type_start..type_end),
                    )),
                    JsdocText::new(
                        &self.body_raw[description_start..],
                        self.body_span
                            .subspan(description_start..self.body_raw.len()),
                    ),
                )
            }
            None => (None, JsdocText::new(self.body_raw, self.body_span)),
        };

        (tag_type, description)
    }

    /// Use for tags like `@param` and `@property`.
    /// Comment can be multiline.
    ///
    /// Variants:
    /// ```text
    /// @kind {type} name comment
    /// @kind {type} name
    /// @kind {type}
    /// @kind name comment
    /// @kind name
    /// @kind
    /// ```
    pub(in crate::jsdoc) fn type_name_comment(
        &self,
    ) -> (
        Option<JsdocTagType<'a>>,
        Option<JsdocTagName<'a>>,
        JsdocText<'a>,
    ) {
        let (tag_type, name_and_description, name_and_description_span) =
            match range::find_type_range(self.body_raw) {
                Some((type_start, type_end)) => {
                    // whitespace after the type is optional
                    let description_start = type_end;
                    (
                        Some(JsdocTagType::new(
                            &self.body_raw[type_start..type_end],
                            self.body_span.subspan(type_start..type_end),
                        )),
                        &self.body_raw[description_start..],
                        self.body_span
                            .subspan(description_start..self.body_raw.len()),
                    )
                }
                None => (None, self.body_raw, self.body_span),
            };

        let (tag_name, description) = match range::find_type_name_range(name_and_description) {
            Some((name_start, name_end)) => {
                // include whitespace for comment trimming
                let description_start = name_end;
                (
                    Some(JsdocTagName::new(
                        &name_and_description[name_start..name_end],
                        name_and_description_span.subspan(name_start..name_end),
                    )),
                    JsdocText::new(
                        &name_and_description[description_start..],
                        name_and_description_span
                            .subspan(description_start..name_and_description.len()),
                    ),
                )
            }
            None => (
                None,
                JsdocText::new(name_and_description, name_and_description_span),
            ),
        };

        (tag_type, tag_name, description)
    }
}
