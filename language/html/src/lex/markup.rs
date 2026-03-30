use super::LexHandler;
use super::buffer::BufferQueue;
use super::lexer::DoctypeIdKind::{Public, System};
use super::lexer::LexerMode::{
    AfterDoctypeIdentifier, AfterDoctypeKeyword, AfterDoctypeName, BeforeDoctypeIdentifier,
    BeforeDoctypeName, BetweenDoctypePublicAndSystemIdentifiers, BogusComment, BogusDoctype,
    CdataSection, CdataSectionBracket, CdataSectionEnd, Comment, CommentEnd, CommentEndBang,
    CommentEndDash, CommentLessThanSign, CommentLessThanSignBang, CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash, CommentStart, CommentStartDash, Data, Doctype,
    DoctypeIdentifierDoubleQuoted, DoctypeIdentifierSingleQuoted, DoctypeName,
    MarkupDeclarationOpen,
};
use super::lexer::{Lexer, ProcessResult};
use super::macros::{eat, eat_exact, get_char, go};

use std::borrow::Cow;

#[allow(clippy::never_loop)]
impl<Parser: LexHandler> Lexer<Parser> {
    /// Step one markup-family lexer state.
    pub(super) fn step_markup_state(&self, input: &BufferQueue) -> ProcessResult<Parser::Handle> {
        match self.state.get() {
            // comment-start-state
            CommentStart => loop {
                match get_char!(self, input) {
                    '-' => go!(self: to CommentStartDash),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_comment '\u{fffd}'; to Comment)
                    }
                    '>' => {
                        self.bad_char_error();
                        go!(self: emit_comment; to Data)
                    }
                    c => go!(self: push_comment c; to Comment),
                }
            },

            // comment-start-dash-state
            CommentStartDash => loop {
                match get_char!(self, input) {
                    '-' => go!(self: to CommentEnd),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: append_comment "-\u{fffd}"; to Comment)
                    }
                    '>' => {
                        self.bad_char_error();
                        go!(self: emit_comment; to Data)
                    }
                    c => go!(self: push_comment '-'; push_comment c; to Comment),
                }
            },

            // comment-state
            Comment => loop {
                match get_char!(self, input) {
                    c @ '<' => go!(self: push_comment c; to CommentLessThanSign),
                    '-' => go!(self: to CommentEndDash),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_comment '\u{fffd}')
                    }
                    c => go!(self: push_comment c),
                }
            },

            // comment-less-than-sign-state
            CommentLessThanSign => loop {
                match get_char!(self, input) {
                    c @ '!' => go!(self: push_comment c; to CommentLessThanSignBang),
                    c @ '<' => go!(self: push_comment c),
                    _ => go!(self: reconsume Comment),
                }
            },

            // comment-less-than-sign-bang-state
            CommentLessThanSignBang => loop {
                match get_char!(self, input) {
                    '-' => go!(self: to CommentLessThanSignBangDash),
                    _ => go!(self: reconsume Comment),
                }
            },

            // comment-less-than-sign-bang-dash-state
            CommentLessThanSignBangDash => loop {
                match get_char!(self, input) {
                    '-' => go!(self: to CommentLessThanSignBangDashDash),
                    _ => go!(self: reconsume CommentEndDash),
                }
            },

            // comment-less-than-sign-bang-dash-dash-state
            CommentLessThanSignBangDashDash => loop {
                match get_char!(self, input) {
                    '>' => go!(self: reconsume CommentEnd),
                    _ => {
                        self.bad_char_error();
                        go!(self: reconsume CommentEnd)
                    }
                }
            },

            // comment-end-dash-state
            CommentEndDash => loop {
                match get_char!(self, input) {
                    '-' => go!(self: to CommentEnd),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: append_comment "-\u{fffd}"; to Comment)
                    }
                    c => go!(self: push_comment '-'; push_comment c; to Comment),
                }
            },

            // comment-end-state
            CommentEnd => loop {
                match get_char!(self, input) {
                    '>' => go!(self: emit_comment; to Data),
                    '!' => go!(self: to CommentEndBang),
                    '-' => go!(self: push_comment '-'),
                    _ => go!(self: append_comment "--"; reconsume Comment),
                }
            },

            // comment-end-bang-state
            CommentEndBang => loop {
                match get_char!(self, input) {
                    '-' => go!(self: append_comment "--!"; to CommentEndDash),
                    '>' => {
                        self.bad_char_error();
                        go!(self: emit_comment; to Data)
                    }
                    '\0' => {
                        self.bad_char_error();
                        go!(self: append_comment "--!\u{fffd}"; to Comment)
                    }
                    c => go!(self: append_comment "--!"; push_comment c; to Comment),
                }
            },

            // doctype-state
            Doctype => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => go!(self: to BeforeDoctypeName),
                    '>' => go!(self: reconsume BeforeDoctypeName),
                    _ => {
                        self.bad_char_error();
                        go!(self: reconsume BeforeDoctypeName)
                    }
                }
            },

            // before-doctype-name-state
            BeforeDoctypeName => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => (),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: create_doctype; push_doctype_name '\u{fffd}'; to DoctypeName)
                    }
                    '>' => {
                        self.bad_char_error();
                        go!(self: create_doctype; force_quirks; emit_doctype; to Data)
                    }
                    c => {
                        go!(self: create_doctype; push_doctype_name (c.to_ascii_lowercase()); to DoctypeName)
                    }
                }
            },

            // doctype-name-state
            DoctypeName => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => go!(self: clear_temp; to AfterDoctypeName),
                    '>' => go!(self: emit_doctype; to Data),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_doctype_name '\u{fffd}')
                    }
                    c => go!(self: push_doctype_name (c.to_ascii_lowercase())),
                }
            },

            // after-doctype-name-state
            AfterDoctypeName => loop {
                if eat!(self, input, "public") {
                    go!(self: to AfterDoctypeKeyword Public);
                } else if eat!(self, input, "system") {
                    go!(self: to AfterDoctypeKeyword System);
                } else {
                    match get_char!(self, input) {
                        '\t' | '\n' | '\x0C' | ' ' => (),
                        '>' => go!(self: emit_doctype; to Data),
                        _ => {
                            self.bad_char_error();
                            go!(self: force_quirks; reconsume BogusDoctype)
                        }
                    }
                }
            },

            // after-doctype-keyword-state
            AfterDoctypeKeyword(kind) => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => go!(self: to BeforeDoctypeIdentifier kind),
                    '"' => {
                        self.bad_char_error();
                        go!(self: clear_doctype_id kind; to DoctypeIdentifierDoubleQuoted kind)
                    }
                    '\'' => {
                        self.bad_char_error();
                        go!(self: clear_doctype_id kind; to DoctypeIdentifierSingleQuoted kind)
                    }
                    '>' => {
                        self.bad_char_error();
                        go!(self: force_quirks; emit_doctype; to Data)
                    }
                    _ => {
                        self.bad_char_error();
                        go!(self: force_quirks; reconsume BogusDoctype)
                    }
                }
            },

            // before-doctype-identifier-state
            BeforeDoctypeIdentifier(kind) => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => (),
                    '"' => go!(self: clear_doctype_id kind; to DoctypeIdentifierDoubleQuoted kind),
                    '\'' => go!(self: clear_doctype_id kind; to DoctypeIdentifierSingleQuoted kind),
                    '>' => {
                        self.bad_char_error();
                        go!(self: force_quirks; emit_doctype; to Data)
                    }
                    _ => {
                        self.bad_char_error();
                        go!(self: force_quirks; reconsume BogusDoctype)
                    }
                }
            },

            // doctype-identifier-double-quoted-state
            DoctypeIdentifierDoubleQuoted(kind) => loop {
                match get_char!(self, input) {
                    '"' => go!(self: to AfterDoctypeIdentifier kind),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_doctype_id kind '\u{fffd}')
                    }
                    '>' => {
                        self.bad_char_error();
                        go!(self: force_quirks; emit_doctype; to Data)
                    }
                    c => go!(self: push_doctype_id kind c),
                }
            },

            // doctype-identifier-single-quoted-state
            DoctypeIdentifierSingleQuoted(kind) => loop {
                match get_char!(self, input) {
                    '\'' => go!(self: to AfterDoctypeIdentifier kind),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_doctype_id kind '\u{fffd}')
                    }
                    '>' => {
                        self.bad_char_error();
                        go!(self: force_quirks; emit_doctype; to Data)
                    }
                    c => go!(self: push_doctype_id kind c),
                }
            },

            // after-doctype-identifier-state
            AfterDoctypeIdentifier(Public) => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => {
                        go!(self: to BetweenDoctypePublicAndSystemIdentifiers)
                    }
                    '>' => go!(self: emit_doctype; to Data),
                    '"' => {
                        self.bad_char_error();
                        go!(self: clear_doctype_id System; to DoctypeIdentifierDoubleQuoted System)
                    }
                    '\'' => {
                        self.bad_char_error();
                        go!(self: clear_doctype_id System; to DoctypeIdentifierSingleQuoted System)
                    }
                    _ => {
                        self.bad_char_error();
                        go!(self: force_quirks; reconsume BogusDoctype)
                    }
                }
            },

            AfterDoctypeIdentifier(System) => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => (),
                    '>' => go!(self: emit_doctype; to Data),
                    _ => {
                        self.bad_char_error();
                        go!(self: reconsume BogusDoctype)
                    }
                }
            },

            // between-doctype-public-and-system-identifiers-state
            BetweenDoctypePublicAndSystemIdentifiers => loop {
                match get_char!(self, input) {
                    '\t' | '\n' | '\x0C' | ' ' => (),
                    '>' => go!(self: emit_doctype; to Data),
                    '"' => {
                        go!(self: clear_doctype_id System; to DoctypeIdentifierDoubleQuoted System)
                    }
                    '\'' => {
                        go!(self: clear_doctype_id System; to DoctypeIdentifierSingleQuoted System)
                    }
                    _ => {
                        self.bad_char_error();
                        go!(self: force_quirks; reconsume BogusDoctype)
                    }
                }
            },

            // bogus-doctype-state
            BogusDoctype => loop {
                match get_char!(self, input) {
                    '>' => go!(self: emit_doctype; to Data),
                    '\0' => {
                        self.bad_char_error();
                    }
                    _ => (),
                }
            },

            // bogus-comment-state
            BogusComment => loop {
                match get_char!(self, input) {
                    '>' => go!(self: emit_comment; to Data),
                    '\0' => {
                        self.bad_char_error();
                        go!(self: push_comment '\u{fffd}')
                    }
                    c => go!(self: push_comment c),
                }
            },

            // markup-declaration-open-state
            MarkupDeclarationOpen => loop {
                if eat_exact!(self, input, "--") {
                    go!(self: clear_comment; to CommentStart);
                } else if eat!(self, input, "doctype") {
                    go!(self: to Doctype);
                } else {
                    if self
                        .parser
                        .adjusted_current_node_present_but_not_in_html_namespace()
                        && eat_exact!(self, input, "[CDATA[")
                    {
                        go!(self: clear_temp; to CdataSection);
                    }
                    self.bad_char_error();
                    go!(self: clear_comment; to BogusComment);
                }
            },

            // cdata-section-state
            CdataSection => loop {
                match get_char!(self, input) {
                    ']' => go!(self: to CdataSectionBracket),
                    '\0' => {
                        self.emit_temp_buf();
                        self.emit_char('\0');
                    }
                    c => go!(self: push_temp c),
                }
            },

            // cdata-section-bracket-state
            CdataSectionBracket => match get_char!(self, input) {
                ']' => go!(self: to CdataSectionEnd),
                _ => go!(self: push_temp ']'; reconsume CdataSection),
            },

            // cdata-section-end-state
            CdataSectionEnd => loop {
                match get_char!(self, input) {
                    ']' => go!(self: push_temp ']'),
                    '>' => {
                        self.emit_temp_buf();
                        go!(self: to Data);
                    }
                    _ => go!(self: push_temp ']'; push_temp ']'; reconsume CdataSection),
                }
            },

            // wrong family
            state => {
                let message = format!("Unexpected markup lexer state {state:?}");
                self.emit_error(Cow::Owned(message));
                ProcessResult::Continue
            }
        }
    }
}
