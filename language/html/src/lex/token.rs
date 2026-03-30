use super::lexer::RawKind;
use super::{Attribute, HtmlString, LocalName, ns};
use std::borrow::Cow;

pub(crate) use self::TagKind::{EndTag, StartTag};
pub(crate) use self::Token::{
    CharacterTokens, CommentToken, DoctypeToken, EOFToken, NullCharacterToken, ParseError, TagToken,
};

/// A `DOCTYPE` token.
#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub(crate) struct Doctype {
    /// The doctype name.
    pub(crate) name: Option<HtmlString>,
    /// The public identifier.
    pub(crate) public_id: Option<HtmlString>,
    /// The system identifier.
    pub(crate) system_id: Option<HtmlString>,
    /// Indicates if this DOCTYPE token should put the document in [quirks mode].
    ///
    /// [quirks mode]: https://dom.spec.whatwg.org/#concept-document-quirks
    pub(crate) force_quirks: bool,
}

/// Whether the tag is a start or an end tag.
#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub(crate) enum TagKind {
    StartTag,
    EndTag,
}

/// A tag token.
#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) struct Tag {
    /// Whether the tag is a start or an end tag.
    pub(crate) kind: TagKind,
    /// The tag local name.
    pub(crate) name: LocalName,
    /// Whether the tag closes itself.
    ///
    /// An example of a self closing tag is `<foo />`.
    pub(crate) self_closing: bool,
    /// The tag attributes.
    pub(crate) attrs: Vec<Attribute>,
    /// Whether duplicate attributes were encountered during tokenization.
    /// This is used for CSP nonce validation - elements with duplicate
    /// attributes are not nonceable per the CSP spec.
    pub(crate) had_duplicate_attributes: bool,
}

impl Tag {
    /// Return whether this is one start tag.
    pub(crate) fn is_start_tag(&self) -> bool {
        self.kind == StartTag
    }

    /// Return whether this is one named start tag.
    pub(crate) fn is_start_tag_named(&self, name: &LocalName) -> bool {
        self.is_start_tag() && self.name == *name
    }

    /// Are the tags equivalent when we don't care about attribute order?
    /// Also ignores the self-closing flag.
    pub(crate) fn equiv_modulo_attr_order(&self, other: &Tag) -> bool {
        if (self.kind != other.kind) || (self.name != other.name) {
            return false;
        }

        let mut self_attrs = self.attrs.clone();
        let mut other_attrs = other.attrs.clone();
        self_attrs.sort();
        other_attrs.sort();

        self_attrs == other_attrs
    }

    /// Return one attribute value by local name.
    pub(crate) fn get_attribute(&self, name: &LocalName) -> Option<HtmlString> {
        self.attrs
            .iter()
            .find(|attribute| attribute.name.ns == ns!() && attribute.name.local == *name)
            .map(|attribute| attribute.value.clone())
    }
}

#[derive(PartialEq, Eq, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Token {
    /// A DOCTYPE declaration like `<!DOCTYPE html>`
    DoctypeToken(Doctype),
    /// A opening or closing tag, like `<foo>` or `</bar>`
    TagToken(Tag),
    /// A comment like `<!-- foo -->`.
    CommentToken(HtmlString),
    /// A sequence of characters.
    CharacterTokens(HtmlString),
    /// A `U+0000 NULL` character in the input.
    NullCharacterToken,
    /// The end of the input stream.
    EOFToken,
    /// One parse error emitted by the lexer.
    ParseError(Cow<'static, str>),
}

/// The result of a [LexHandler] consuming a single token.
#[derive(Debug, PartialEq)]
#[must_use]
pub(crate) enum LexerAction<Handle> {
    /// The tokenizer can continue parsing the input as usual.
    Continue,
    /// The parser has completed parsing a `<script>` tag, blocking the lexer
    /// until the script is executed.
    Script(Handle),
    /// The tokenizer should set its state to the [PLAINTEXT state](https://html.spec.whatwg.org/#plaintext-state).
    Plaintext,
    /// The tokenizer should set its state to the given rawdata state.
    RawData(RawKind),
    /// The document indicated that the given encoding should be used to parse it.
    ///
    /// HTML5-compatible implementations should parse the encoding label using the algorithm
    /// described in <https://encoding.spec.whatwg.org/#concept-encoding-get>. The label
    /// has not been validated by html5ever. Invalid or unknown encodings can be ignored.
    ///
    /// If the decoder is confident that the current encoding is correct then this message
    /// can safely be ignored.
    EncodingIndicator(HtmlString),
}

/// One lexer step result.
#[must_use]
#[derive(Debug, PartialEq)]
pub(crate) enum LexerResult<Handle> {
    /// The lexer consumed the available input.
    Done,
    /// The lexer was blocked by one script token.
    Script(Handle),
    /// The lexer found one encoding declaration.
    EncodingIndicator(HtmlString),
}

/// Types which can receive tokens from the lexer.
pub(crate) trait LexHandler {
    /// The type of a DOM node.
    type Handle;

    /// Process a token.
    fn process_token(&self, token: Token, line_number: u64) -> LexerAction<Self::Handle>;

    /// Signal that tokenization reached the end of the document.
    fn end(&self) {}

    /// Used in the [markup declaration open state]. By default, this always
    /// returns false and thus all CDATA sections are tokenized as bogus
    /// comments.
    ///
    /// [markup declaration open state]: https://html.spec.whatwg.org/multipage/#markup-declaration-open-state
    fn adjusted_current_node_present_but_not_in_html_namespace(&self) -> bool {
        false
    }
}
