use super::lexer::LexerMode::{AttributeValue, Data, Plaintext, RawData};
use super::lexer::{DoctypeIdKind, Lexer, LexerMode, ProcessResult};
use super::token::{
    CharacterTokens, CommentToken, DoctypeToken, EOFToken, EndTag, LexerAction, NullCharacterToken,
    ParseError, StartTag, Tag, TagKind, TagToken, Token,
};
use super::{Attribute, HtmlString, LexHandler, LocalName, QualifiedName, ns};

use std::borrow::Cow::{self, Borrowed};
use std::cell::RefMut;
use std::mem;

impl<Parser: LexHandler> Lexer<Parser> {
    /// Forward one token into the parser.
    fn process_token(&self, token: Token) -> LexerAction<Parser::Handle> {
        if self.options.profile {
            let (result, elapsed_nanoseconds) =
                super::macros::time!(self.parser.process_token(token, self.current_line.get()));
            self.time_in_parser
                .set(self.time_in_parser.get() + elapsed_nanoseconds);
            result
        } else {
            self.parser.process_token(token, self.current_line.get())
        }
    }

    /// Forward one token and require normal lexer continuation.
    fn process_token_and_continue(&self, token: Token) {
        if !matches!(self.process_token(token), LexerAction::Continue) {
            self.emit_error(Borrowed("Unexpected lexer parser transition"));
        }
    }

    /// Emit one bad-character parse error.
    #[inline]
    pub(super) fn bad_char_error(&self) {
        let message = if self.options.exact_errors {
            Cow::from("Bad character")
        } else {
            let character = self.current_char.get();
            let state = self.state.get();
            Cow::from(format!("Saw {character} in state {state:?}"))
        };

        self.emit_error(message);
    }

    /// Emit one unexpected-EOF parse error.
    #[inline]
    pub(super) fn bad_eof_error(&self) {
        let message = if self.options.exact_errors {
            Cow::from("Unexpected EOF")
        } else {
            let state = self.state.get();
            Cow::from(format!("Saw EOF in state {state:?}"))
        };

        self.emit_error(message);
    }

    /// Emit one character token.
    pub(super) fn emit_char(&self, character: char) {
        let token = match character {
            '\0' => NullCharacterToken,
            _ => CharacterTokens(HtmlString::from_char(character)),
        };

        self.process_token_and_continue(token);
    }

    /// Emit one character run without embedded nulls.
    pub(super) fn emit_chars(&self, characters: HtmlString) {
        self.process_token_and_continue(CharacterTokens(characters));
    }

    /// Emit the current tag token and apply any parser-directed state change.
    pub(super) fn emit_current_tag(&self) -> ProcessResult<Parser::Handle> {
        self.finish_attribute();

        // finish the tag token
        let name = LocalName::from(&**self.current_tag_name.borrow());
        self.current_tag_name.borrow_mut().clear();

        match self.current_tag_kind.get() {
            StartTag => {
                *self.last_start_tag_name.borrow_mut() = Some(name);
            }
            EndTag => {
                if !self.current_tag_attrs.borrow().is_empty() {
                    self.emit_error(Borrowed("Attributes on an end tag"));
                }

                if self.current_tag_self_closing.get() {
                    self.emit_error(Borrowed("Self-closing end tag"));
                }
            }
        }

        let token = TagToken(Tag {
            kind: self.current_tag_kind.get(),
            name,
            self_closing: self.current_tag_self_closing.get(),
            attrs: mem::take(&mut self.current_tag_attrs.borrow_mut()),
            had_duplicate_attributes: self.current_tag_had_duplicate_attributes.get(),
        });

        // apply the parser response
        match self.process_token(token) {
            LexerAction::Continue => ProcessResult::Continue,
            LexerAction::Plaintext => {
                self.state.set(Plaintext);
                ProcessResult::Continue
            }
            LexerAction::Script(node) => {
                self.state.set(Data);
                ProcessResult::Script(node)
            }
            LexerAction::RawData(kind) => {
                self.state.set(RawData(kind));
                ProcessResult::Continue
            }
            LexerAction::EncodingIndicator(encoding) => ProcessResult::EncodingIndicator(encoding),
        }
    }

    /// Emit the buffered temporary text.
    pub(super) fn emit_temp_buf(&self) {
        // clear the buffered text after emission
        let buffered_text = mem::take(&mut *self.temp_buf.borrow_mut());
        self.emit_chars(buffered_text);
    }

    /// Clear the temporary buffer without reallocating it.
    pub(super) fn clear_temp_buf(&self) {
        // reuse allocation
        self.temp_buf.borrow_mut().clear();
    }

    /// Emit the current comment token.
    pub(super) fn emit_current_comment(&self) {
        let comment = mem::take(&mut *self.current_comment.borrow_mut());
        self.process_token_and_continue(CommentToken(comment));
    }

    /// Discard the current tag under construction.
    pub(super) fn discard_tag(&self) {
        self.current_tag_name.borrow_mut().clear();
        self.current_tag_self_closing.set(false);
        self.current_tag_had_duplicate_attributes.set(false);
        *self.current_tag_attrs.borrow_mut() = vec![];
    }

    /// Start one new tag token with one first name character.
    pub(super) fn create_tag(&self, kind: TagKind, character: char) {
        self.discard_tag();
        self.current_tag_name.borrow_mut().push_char(character);
        self.current_tag_kind.set(kind);
    }

    /// Return whether the current end tag matches the last emitted start tag.
    pub(super) fn have_appropriate_end_tag(&self) -> bool {
        match self.last_start_tag_name.borrow().as_ref() {
            Some(last_start_tag_name) => {
                self.current_tag_kind.get() == EndTag
                    && last_start_tag_name.eq_str(&self.current_tag_name.borrow())
            }
            None => false,
        }
    }

    /// Start one new attribute with one first name character.
    pub(super) fn create_attribute(&self, character: char) {
        self.finish_attribute();
        self.current_attr_name.borrow_mut().push_char(character);
    }

    /// Finish the current attribute and attach it to the current tag.
    fn finish_attribute(&self) {
        if self.current_attr_name.borrow().is_empty() {
            return;
        }

        // duplicate attributes
        // the spec reports this earlier, but the final outcome is the same here
        let is_duplicate = {
            let attribute_name = &*self.current_attr_name.borrow();
            self.current_tag_attrs
                .borrow()
                .iter()
                .any(|attribute| attribute.name.local.eq_str(attribute_name))
        };

        if is_duplicate {
            self.emit_error(Borrowed("Duplicate attribute"));
            self.current_tag_had_duplicate_attributes.set(true);
            self.current_attr_name.borrow_mut().clear();
            self.current_attr_value.borrow_mut().clear();
        } else {
            let name = LocalName::from(&**self.current_attr_name.borrow());
            self.current_attr_name.borrow_mut().clear();

            self.current_tag_attrs.borrow_mut().push(Attribute {
                // foreign-element namespace fixup happens later in the parser
                name: QualifiedName::new(None, ns!(), name),
                value: mem::take(&mut self.current_attr_value.borrow_mut()),
            });
        }
    }

    /// Emit the current doctype token.
    pub(super) fn emit_current_doctype(&self) {
        let doctype = self.current_doctype.take();
        self.process_token_and_continue(DoctypeToken(doctype));
    }

    /// Borrow one doctype id field by kind.
    pub(super) fn doctype_id(&self, kind: DoctypeIdKind) -> RefMut<'_, Option<HtmlString>> {
        let current_doctype = self.current_doctype.borrow_mut();
        match kind {
            DoctypeIdKind::Public => RefMut::map(current_doctype, |doctype| &mut doctype.public_id),
            DoctypeIdKind::System => RefMut::map(current_doctype, |doctype| &mut doctype.system_id),
        }
    }

    /// Clear one doctype id field without reallocating it.
    pub(super) fn clear_doctype_id(&self, kind: DoctypeIdKind) {
        let mut identifier = self.doctype_id(kind);
        match *identifier {
            Some(ref mut string) => string.clear(),
            None => *identifier = Some(HtmlString::new()),
        }
    }

    /// Push one character into one optional string buffer.
    pub(super) fn push_optional_string(&self, value: &mut Option<HtmlString>, character: char) {
        match *value {
            Some(ref mut string) => string.push_char(character),
            None => *value = Some(HtmlString::from_char(character)),
        }
    }

    /// Start tokenizing one character reference.
    pub(super) fn start_consuming_character_reference(&self) {
        debug_assert!(
            self.character_reference_tokenizer.borrow().is_none(),
            "Nested character references are impossible"
        );

        let is_in_attribute = matches!(self.state.get(), AttributeValue(_));
        *self.character_reference_tokenizer.borrow_mut() = Some(
            super::reference::CharacterReferenceTokenizer::new(is_in_attribute),
        );
    }

    /// Emit one EOF token.
    pub(super) fn emit_eof(&self) {
        self.process_token_and_continue(EOFToken);
    }

    /// Transition the lexer into one new state.
    pub(super) fn transition_to(&self, state: LexerMode) {
        self.state.set(state);
    }

    /// Reconsume the current character in the next state.
    pub(super) fn reconsume_next(&self) {
        self.reconsume.set(true);
    }

    /// Push one character into the current tag name.
    pub(super) fn push_tag_name(&self, character: char) {
        self.current_tag_name.borrow_mut().push_char(character);
    }

    /// Push one character into the temporary buffer.
    pub(super) fn push_temp_char(&self, character: char) {
        self.temp_buf.borrow_mut().push_char(character);
    }

    /// Push one character into the current attribute name.
    pub(super) fn push_attribute_name(&self, character: char) {
        self.current_attr_name.borrow_mut().push_char(character);
    }

    /// Push one character into the current attribute value.
    pub(super) fn push_attribute_value(&self, character: char) {
        self.current_attr_value.borrow_mut().push_char(character);
    }

    /// Append one string buffer to the current attribute value.
    pub(super) fn append_attribute_value<T>(&self, value: T)
    where
        T: AsRef<str>,
    {
        self.current_attr_value.borrow_mut().push_html_string(value);
    }

    /// Push one character into the current comment buffer.
    pub(super) fn push_comment_char(&self, character: char) {
        self.current_comment.borrow_mut().push_char(character);
    }

    /// Append one string slice to the current comment buffer.
    pub(super) fn append_comment_text(&self, value: &str) {
        self.current_comment.borrow_mut().push_slice(value);
    }

    /// Clear the current comment buffer.
    pub(super) fn clear_comment(&self) {
        self.current_comment.borrow_mut().clear();
    }

    /// Initialize one fresh doctype token.
    pub(super) fn initialize_doctype(&self) {
        *self.current_doctype.borrow_mut() = super::token::Doctype::default();
    }

    /// Push one character into the current doctype name.
    pub(super) fn push_doctype_name(&self, character: char) {
        self.push_optional_string(&mut self.current_doctype.borrow_mut().name, character);
    }

    /// Push one character into one current doctype identifier.
    pub(super) fn push_doctype_identifier(&self, kind: DoctypeIdKind, character: char) {
        self.push_optional_string(&mut self.doctype_id(kind), character);
    }

    /// Force quirks mode on the current doctype token.
    pub(super) fn force_quirks(&self) {
        self.current_doctype.borrow_mut().force_quirks = true;
    }

    /// Emit one parse error token.
    pub(super) fn emit_error(&self, error: Cow<'static, str>) {
        self.process_token_and_continue(ParseError(error));
    }
}
