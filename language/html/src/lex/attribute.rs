use super::buffer::{BufferQueue, FromSet, NotFromSet};
use super::lexer::AttrValueKind::{DoubleQuoted, SingleQuoted, Unquoted};
use super::lexer::LexerMode::{
    AfterAttributeName, AfterAttributeValueQuoted, AttributeName, AttributeValue,
    BeforeAttributeName, BeforeAttributeValue, Data, SelfClosingStartTag,
};
use super::lexer::{Lexer, ProcessResult};
use super::macros::{get_char, go, peek, pop_except_from, small_char_set};
use super::{LexHandler, lower_ascii_letter};

use std::borrow::Cow;

#[allow(clippy::never_loop)]
impl<Parser: LexHandler> Lexer<Parser> {
    /// Step one attribute-family lexer state.
    pub(super) fn step_attribute_state(
        &self,
        input: &BufferQueue,
    ) -> ProcessResult<Parser::Handle> {
        match self.state.get() {
            // before-attribute-name-state
            BeforeAttributeName => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => (),
                    '/' => go!(self: to SelfClosingStartTag),
                    '>' => go!(self: emit_tag Data),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: create_attr '\u{fffd}'; to AttributeName)
                    }
                    c => match lower_ascii_letter(c) {
                        Some(lower_case) => go!(self: create_attr lower_case; to AttributeName),
                        None => {
                            if matches!(c, '"' | '\'' | '<' | '=') {
                                self.bad_char_error();
                            }

                            go!(self: create_attr c; to AttributeName);
                        }
                    },
                }
            },

            // attribute-name-state
            AttributeName => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => go!(self: to AfterAttributeName),
                    '/' => go!(self: to SelfClosingStartTag),
                    '=' => go!(self: to BeforeAttributeValue),
                    '>' => go!(self: emit_tag Data),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_name '\u{fffd}')
                    }
                    c => match lower_ascii_letter(c) {
                        Some(lower_case) => go!(self: push_name lower_case),
                        None => {
                            if matches!(c, '"' | '\'' | '<') {
                                self.bad_char_error();
                            }

                            go!(self: push_name c);
                        }
                    },
                }
            },

            // after-attribute-name-state
            AfterAttributeName => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => (),
                    '/' => go!(self: to SelfClosingStartTag),
                    '=' => go!(self: to BeforeAttributeValue),
                    '>' => go!(self: emit_tag Data),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: create_attr '\u{fffd}'; to AttributeName)
                    }
                    c => match lower_ascii_letter(c) {
                        Some(lower_case) => go!(self: create_attr lower_case; to AttributeName),
                        None => {
                            if matches!(c, '"' | '\'' | '<') {
                                self.bad_char_error();
                            }

                            go!(self: create_attr c; to AttributeName);
                        }
                    },
                }
            },

            // before-attribute-value-state
            // peek keeps the first attribute byte on the normal path
            BeforeAttributeValue => loop {
                match peek!(self, input) {
                    '\t' | '\n' | '\r' | '\x0C' | ' ' => go!(self: discard_char input),
                    '"' => go!(self: discard_char input; to AttributeValue DoubleQuoted),
                    '\'' => go!(self: discard_char input; to AttributeValue SingleQuoted),
                    '>' => {
                        go!(self: discard_char input);
                        self.bad_char_error();
                        go!(self: emit_tag Data)
                    }
                    _ => go!(self: to AttributeValue Unquoted),
                }
            },

            // attribute-value-double-quoted-state
            AttributeValue(DoubleQuoted) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '"' '&' '\0' '\n')) {
                    FromSet('"') => go!(self: to AfterAttributeValueQuoted),
                    FromSet('&') => go!(self: consume_char_ref),
                    FromSet('\0') => {
                        self.bad_char_error();
                        go!(self: push_value '\u{fffd}')
                    }
                    FromSet(c) => go!(self: push_value c),
                    NotFromSet(ref buffer) => go!(self: append_value buffer),
                }
            },

            // attribute-value-single-quoted-state
            AttributeValue(SingleQuoted) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\'' '&' '\0' '\n')) {
                    FromSet('\'') => go!(self: to AfterAttributeValueQuoted),
                    FromSet('&') => go!(self: consume_char_ref),
                    FromSet('\0') => {
                        self.bad_char_error();
                        go!(self: push_value '\u{fffd}')
                    }
                    FromSet(c) => go!(self: push_value c),
                    NotFromSet(ref buffer) => go!(self: append_value buffer),
                }
            },

            // attribute-value-unquoted-state
            AttributeValue(Unquoted) => loop {
                match pop_except_from!(
                    self,
                    input,
                    small_char_set!('\r' '\t' '\n' '\x0C' ' ' '&' '>' '\0')
                ) {
                    FromSet('\t') | FromSet('\n') | FromSet('\x0C') | FromSet(' ') => {
                        go!(self: to BeforeAttributeName)
                    }
                    FromSet('&') => go!(self: consume_char_ref),
                    FromSet('>') => go!(self: emit_tag Data),
                    FromSet('\0') => {
                        self.bad_char_error();
                        go!(self: push_value '\u{fffd}')
                    }
                    FromSet(c) => {
                        if matches!(c, '"' | '\'' | '<' | '=' | '`') {
                            self.bad_char_error();
                        }

                        go!(self: push_value c);
                    }
                    NotFromSet(ref buffer) => go!(self: append_value buffer),
                }
            },

            // after-attribute-value-quoted-state
            AfterAttributeValueQuoted => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => go!(self: to BeforeAttributeName),
                    '/' => go!(self: to SelfClosingStartTag),
                    '>' => go!(self: emit_tag Data),
                    _ => {
                        self.bad_char_error();
                        go!(self: reconsume BeforeAttributeName)
                    }
                }
            },

            // self-closing-start-tag-state
            SelfClosingStartTag => loop {
                match get_char!(self, input) {
                    '>' => {
                        self.current_tag_self_closing.set(true);
                        go!(self: emit_tag Data);
                    }
                    _ => {
                        self.bad_char_error();
                        go!(self: reconsume BeforeAttributeName)
                    }
                }
            },

            // wrong family
            state => {
                let message = format!("Unexpected attribute lexer state {state:?}");
                self.emit_error(Cow::Owned(message));
                ProcessResult::Continue
            }
        }
    }
}
