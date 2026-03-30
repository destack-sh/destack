use super::TagKind::{EndTag, StartTag};
use super::buffer::BufferQueue;
use super::lexer::LexerMode::{
    BeforeAttributeName, BogusComment, Data, EndTagOpen, MarkupDeclarationOpen,
    SelfClosingStartTag, TagName, TagOpen,
};
use super::lexer::{Lexer, ProcessResult};
use super::macros::{get_char, go};
use super::{LexHandler, lower_ascii_letter};

use std::borrow::Cow;

impl<Parser: LexHandler> Lexer<Parser> {
    /// Step one tag-family lexer state.
    pub(super) fn step_tag_state(&self, input: &BufferQueue) -> ProcessResult<Parser::Handle> {
        match self.state.get() {
            // tag-open-state
            TagOpen => match get_char!(self, input) {
                '!' => go!(self: to MarkupDeclarationOpen),
                '/' => go!(self: to EndTagOpen),
                '?' => {
                    self.bad_char_error();
                    go!(self: clear_comment; reconsume BogusComment)
                }
                c => match lower_ascii_letter(c) {
                    Some(lower_case) => go!(self: create_tag StartTag lower_case; to TagName),
                    None => {
                        self.bad_char_error();
                        self.emit_char('<');
                        go!(self: reconsume Data)
                    }
                },
            },

            // end-tag-open-state
            EndTagOpen => match get_char!(self, input) {
                '>' => {
                    self.bad_char_error();
                    go!(self: to Data)
                }
                c => match lower_ascii_letter(c) {
                    Some(lower_case) => go!(self: create_tag EndTag lower_case; to TagName),
                    None => {
                        self.bad_char_error();
                        go!(self: clear_comment; reconsume BogusComment)
                    }
                },
            },

            // tag-name-state
            TagName => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        go!(self: to BeforeAttributeName)
                    }
                    '/' => go!(self: to SelfClosingStartTag),
                    '>' => go!(self: emit_tag Data),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_tag '\u{fffd}')
                    }
                    c => go!(self: push_tag (c.to_ascii_lowercase())),
                }
            },

            // wrong family
            state => {
                let message = format!("Unexpected tag lexer state {state:?}");
                self.emit_error(Cow::Owned(message));
                ProcessResult::Continue
            }
        }
    }
}
