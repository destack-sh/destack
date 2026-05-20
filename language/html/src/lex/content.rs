use super::TagKind::EndTag;
use super::buffer::{BufferQueue, FromSet, NotFromSet};
use super::lexer::LexerMode::{
    BeforeAttributeName, Data, Plaintext, RawData, RawEndTagName, RawEndTagOpen, RawLessThanSign,
    ScriptDataDoubleEscapeEnd, ScriptDataEscapeStart, ScriptDataEscapeStartDash,
    ScriptDataEscapedDash, ScriptDataEscapedDashDash, SelfClosingStartTag, TagOpen,
};
use super::lexer::RawKind::{Rawtext, Rcdata, ScriptData, ScriptDataEscaped};
use super::lexer::ScriptEscapeKind::{DoubleEscaped, Escaped};
use super::lexer::{Lexer, ProcessResult};
use super::macros::{get_char, go, pop_except_from, small_char_set};
use super::{LexHandler, lower_ascii_letter};

use std::borrow::Cow;

#[allow(clippy::never_loop)]
impl<Parser: LexHandler> Lexer<Parser> {
    /// Step one content-family lexer state.
    pub(super) fn step_content_state(&self, input: &BufferQueue) -> ProcessResult<Parser::Handle> {
        match self.state.get() {
            // data-state
            Data => loop {
                let set = small_char_set!('\r' '\0' '&' '<' '\n');

                #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
                let set_result = if !(self.options.exact_errors
                    || self.reconsume.get()
                    || self.ignore_lf.get())
                    && Self::is_supported_simd_feature_detected()
                {
                    let front_buffer = input.peek_front_chunk_mut();
                    let Some(mut front_buffer) = front_buffer else {
                        return ProcessResult::Suspend;
                    };

                    // fast path guard
                    // the simd path is not worth taking when the first byte already matches
                    let Some(first_char) = front_buffer.chars().next() else {
                        drop(front_buffer);
                        input.pop_front();
                        return ProcessResult::Continue;
                    };

                    if matches!(first_char, '\r' | '\0' | '&' | '<' | '\n') {
                        drop(front_buffer);
                        self.pop_except_from(input, set)
                    } else {
                        // SAFETY: the runtime feature check above guarantees simd support
                        let result = unsafe { self.data_state_simd_fast_path(&mut front_buffer) };

                        if front_buffer.is_empty() {
                            drop(front_buffer);
                            input.pop_front();
                        }

                        result
                    }
                } else {
                    self.pop_except_from(input, set)
                };

                #[cfg(not(any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                    target_arch = "aarch64"
                )))]
                let set_result = self.pop_except_from(input, set);

                let Some(set_result) = set_result else {
                    return ProcessResult::Suspend;
                };
                match set_result {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\0');
                    }
                    FromSet('&') => go!(self: consume_char_ref),
                    FromSet('<') => go!(self: to TagOpen),
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // rcdata-state
            RawData(Rcdata) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\0' '&' '<' '\n')) {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                    }
                    FromSet('&') => go!(self: consume_char_ref),
                    FromSet('<') => go!(self: to RawLessThanSign Rcdata),
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // rawtext-state
            RawData(Rawtext) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\0' '<' '\n')) {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                    }
                    FromSet('<') => go!(self: to RawLessThanSign Rawtext),
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // script-data-state
            RawData(ScriptData) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\0' '<' '\n')) {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                    }
                    FromSet('<') => go!(self: to RawLessThanSign ScriptData),
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // script-data-escaped-state
            RawData(ScriptDataEscaped(Escaped)) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\0' '-' '<' '\n')) {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                    }
                    FromSet('-') => {
                        self.emit_char('-');
                        go!(self: to ScriptDataEscapedDash Escaped);
                    }
                    FromSet('<') => go!(self: to RawLessThanSign ScriptDataEscaped Escaped),
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // script-data-double-escaped-state
            RawData(ScriptDataEscaped(DoubleEscaped)) => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\0' '-' '<' '\n')) {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                    }
                    FromSet('-') => {
                        self.emit_char('-');
                        go!(self: to ScriptDataEscapedDash DoubleEscaped);
                    }
                    FromSet('<') => {
                        self.emit_char('<');
                        go!(self: to RawLessThanSign ScriptDataEscaped DoubleEscaped)
                    }
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // plaintext-state
            Plaintext => loop {
                match pop_except_from!(self, input, small_char_set!('\r' '\0' '\n')) {
                    FromSet('\0') => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                    }
                    FromSet(c) => self.emit_char(c),
                    NotFromSet(buffer) => self.emit_chars(buffer),
                }
            },

            // script-data-escaped-less-than-sign-state
            RawLessThanSign(ScriptDataEscaped(Escaped)) => loop {
                match get_char!(self, input) {
                    '/' => go!(self: clear_temp; to RawEndTagOpen ScriptDataEscaped Escaped),
                    c => match lower_ascii_letter(c) {
                        Some(lower_case) => {
                            go!(self: clear_temp; push_temp lower_case);
                            self.emit_char('<');
                            self.emit_char(c);
                            go!(self: to ScriptDataEscapeStart DoubleEscaped);
                        }
                        None => {
                            self.emit_char('<');
                            go!(self: reconsume RawData ScriptDataEscaped Escaped);
                        }
                    },
                }
            },

            // script-data-double-escaped-less-than-sign-state
            RawLessThanSign(ScriptDataEscaped(DoubleEscaped)) => loop {
                match get_char!(self, input) {
                    '/' => {
                        go!(self: clear_temp);
                        self.emit_char('/');
                        go!(self: to ScriptDataDoubleEscapeEnd);
                    }
                    _ => go!(self: reconsume RawData ScriptDataEscaped DoubleEscaped),
                }
            },

            // raw-less-than-sign-state
            RawLessThanSign(kind) => loop {
                match get_char!(self, input) {
                    '/' => go!(self: clear_temp; to RawEndTagOpen kind),
                    '!' if kind == ScriptData => {
                        self.emit_char('<');
                        self.emit_char('!');
                        go!(self: to ScriptDataEscapeStart Escaped);
                    }
                    _ => {
                        self.emit_char('<');
                        go!(self: reconsume RawData kind);
                    }
                }
            },

            // raw-end-tag-open-state
            RawEndTagOpen(kind) => loop {
                let c = get_char!(self, input);
                match lower_ascii_letter(c) {
                    Some(lower_case) => {
                        go!(self: create_tag EndTag lower_case; push_temp c; to RawEndTagName kind)
                    }
                    None => {
                        self.emit_char('<');
                        self.emit_char('/');
                        go!(self: reconsume RawData kind);
                    }
                }
            },

            // raw-end-tag-name-state
            RawEndTagName(kind) => loop {
                let c = get_char!(self, input);
                if self.have_appropriate_end_tag() {
                    match c {
                        '\t' | '\n' | '\x0C' | ' ' => go!(self: clear_temp; to BeforeAttributeName),
                        '/' => go!(self: clear_temp; to SelfClosingStartTag),
                        '>' => go!(self: clear_temp; emit_tag Data),
                        _ => (),
                    }
                }

                match lower_ascii_letter(c) {
                    Some(lower_case) => go!(self: push_tag lower_case; push_temp c),
                    None => {
                        go!(self: discard_tag);
                        self.emit_char('<');
                        self.emit_char('/');
                        self.emit_temp_buf();
                        go!(self: reconsume RawData kind);
                    }
                }
            },

            // script-data-double-escape-start-state
            ScriptDataEscapeStart(DoubleEscaped) => loop {
                let c = get_char!(self, input);
                match c {
                    '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                        let escape_kind = if &**self.temp_buf.borrow() == "script" {
                            DoubleEscaped
                        } else {
                            Escaped
                        };
                        self.emit_char(c);
                        go!(self: to RawData ScriptDataEscaped escape_kind);
                    }
                    _ => match lower_ascii_letter(c) {
                        Some(lower_case) => {
                            go!(self: push_temp lower_case);
                            self.emit_char(c);
                        }
                        None => go!(self: reconsume RawData ScriptDataEscaped Escaped),
                    },
                }
            },

            // script-data-escape-start-state
            ScriptDataEscapeStart(Escaped) => loop {
                match get_char!(self, input) {
                    '-' => {
                        self.emit_char('-');
                        go!(self: to ScriptDataEscapeStartDash);
                    }
                    _ => go!(self: reconsume RawData ScriptData),
                }
            },

            // script-data-escape-start-dash-state
            ScriptDataEscapeStartDash => loop {
                match get_char!(self, input) {
                    '-' => {
                        self.emit_char('-');
                        go!(self: to ScriptDataEscapedDashDash Escaped);
                    }
                    _ => go!(self: reconsume RawData ScriptData),
                }
            },

            // script-data-escaped-dash-state
            ScriptDataEscapedDash(kind) => loop {
                match get_char!(self, input) {
                    '-' => {
                        self.emit_char('-');
                        go!(self: to ScriptDataEscapedDashDash kind);
                    }
                    '<' => {
                        if kind == DoubleEscaped {
                            self.emit_char('<');
                        }
                        go!(self: to RawLessThanSign ScriptDataEscaped kind);
                    }
                    '\0' => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                        go!(self: to RawData ScriptDataEscaped kind)
                    }
                    c => {
                        self.emit_char(c);
                        go!(self: to RawData ScriptDataEscaped kind);
                    }
                }
            },

            // script-data-escaped-dash-dash-state
            ScriptDataEscapedDashDash(kind) => loop {
                match get_char!(self, input) {
                    '-' => {
                        self.emit_char('-');
                    }
                    '<' => {
                        if kind == DoubleEscaped {
                            self.emit_char('<');
                        }
                        go!(self: to RawLessThanSign ScriptDataEscaped kind);
                    }
                    '>' => {
                        self.emit_char('>');
                        go!(self: to RawData ScriptData);
                    }
                    '\0' => {
                        self.bad_char_error();
                        self.emit_char('\u{fffd}');
                        go!(self: to RawData ScriptDataEscaped kind)
                    }
                    c => {
                        self.emit_char(c);
                        go!(self: to RawData ScriptDataEscaped kind);
                    }
                }
            },

            // script-data-double-escape-end-state
            ScriptDataDoubleEscapeEnd => loop {
                let c = get_char!(self, input);
                match c {
                    '\t' | '\n' | '\x0C' | ' ' | '/' | '>' => {
                        let escape_kind = if &**self.temp_buf.borrow() == "script" {
                            Escaped
                        } else {
                            DoubleEscaped
                        };
                        self.emit_char(c);
                        go!(self: to RawData ScriptDataEscaped escape_kind);
                    }
                    _ => match lower_ascii_letter(c) {
                        Some(lower_case) => {
                            go!(self: push_temp lower_case);
                            self.emit_char(c);
                        }
                        None => go!(self: reconsume RawData ScriptDataEscaped DoubleEscaped),
                    },
                }
            },

            // wrong family
            state => {
                let message = format!("Unexpected content lexer state {state:?}");
                self.emit_error(Cow::Owned(message));
                ProcessResult::Continue
            }
        }
    }
}
