use crate::lex::{
    CharacterTokens, CommentToken, Doctype, DoctypeToken, EOFToken, ExpandedName, HtmlString,
    LexHandler, LexerAction, LocalName, NullCharacterToken, ParseError, RawKind, StartTag, Tag,
    TagToken, Token as LexerToken, local_name, ns, to_escaped_string,
};

use super::builder::Handle;
use super::doctype;
use super::parser::{InsertionMode, Parser, ProcessResult, QuirksMode};
use super::tag::declare_tag_set;
use super::token::{SplitStatus, Token};

use std::borrow::Cow::{self, Borrowed};
use std::cell::Ref;
use std::collections::VecDeque;
use std::fmt;

impl Parser<'_> {
    /// Drive one parser token until it either finishes or changes lexer mode.
    fn process_to_completion(&self, mut token: Token) -> LexerAction<Handle> {
        let mut pending_tokens = VecDeque::new();

        loop {
            // self-closing bookkeeping
            let should_acknowledge_self_closing = matches!(
                token,
                Token::Tag(Tag {
                    self_closing: true,
                    kind: StartTag,
                    ..
                })
            );

            // step in the right tree-construction path
            let result = if self.is_foreign(&token) {
                self.step_foreign(token)
            } else {
                let mode = self.mode.get();
                self.step(mode, token)
            };

            // process one parser result
            match result {
                ProcessResult::Done => {
                    if should_acknowledge_self_closing {
                        self.builder
                            .parse_error(Borrowed("Unacknowledged self-closing tag"));
                    }

                    let Some(next_token) = pending_tokens.pop_front() else {
                        return LexerAction::Continue;
                    };

                    token = next_token;
                }

                ProcessResult::DoneAckSelfClosing => {
                    let Some(next_token) = pending_tokens.pop_front() else {
                        return LexerAction::Continue;
                    };

                    token = next_token;
                }

                ProcessResult::Reprocess(mode, next_token) => {
                    self.mode.set(mode);
                    token = next_token;
                }

                ProcessResult::SplitWhitespace(mut text) => {
                    let Some((first, is_whitespace)) =
                        text.pop_front_char_run(|character| character.is_ascii_whitespace())
                    else {
                        return LexerAction::Continue;
                    };

                    let split_status = if is_whitespace {
                        SplitStatus::Whitespace
                    } else {
                        SplitStatus::NotWhitespace
                    };
                    token = Token::Characters(split_status, first);

                    if text.len32() > 0 {
                        pending_tokens.push_back(Token::Characters(SplitStatus::NotSplit, text));
                    }
                }

                ProcessResult::Script(node) => {
                    self.report_pending_tokens(
                        &mut pending_tokens,
                        "Parser queued extra tokens before script handoff",
                    );
                    return LexerAction::Script(node);
                }

                ProcessResult::ToPlaintext => {
                    self.report_pending_tokens(
                        &mut pending_tokens,
                        "Parser queued extra tokens before plaintext handoff",
                    );
                    return LexerAction::Plaintext;
                }

                ProcessResult::ToRawData(kind) => {
                    self.report_pending_tokens(
                        &mut pending_tokens,
                        "Parser queued extra tokens before raw data",
                    );
                    return LexerAction::RawData(kind);
                }

                ProcessResult::EncodingIndicator(encoding) => {
                    return LexerAction::EncodingIndicator(encoding);
                }
            }
        }
    }

    /// Report and clear any queued parser tokens.
    fn report_pending_tokens(&self, pending_tokens: &mut VecDeque<Token>, message: &'static str) {
        if pending_tokens.is_empty() {
            return;
        }

        self.builder.parse_error(Cow::from(message));
        pending_tokens.clear();
    }
}

impl LexHandler for Parser<'_> {
    type Handle = Handle;

    /// Process one lexer token.
    fn process_token(&self, token: LexerToken, line_number: u64) -> LexerAction<Handle> {
        // source line tracking
        if line_number != self.current_line.get() {
            self.builder.set_current_line(line_number);
        }

        // lexer token lowering
        let ignore_lf = self.ignore_lf.take();
        let token = match token {
            ParseError(error) => {
                self.builder.parse_error(error);
                return LexerAction::Continue;
            }

            DoctypeToken(doctype) => {
                return self.process_doctype(doctype);
            }

            TagToken(tag) => Token::Tag(tag),
            CommentToken(text) => Token::Comment(text),
            NullCharacterToken => Token::NullCharacter,
            EOFToken => Token::Eof,

            CharacterTokens(mut text) => {
                if ignore_lf && text.starts_with("\n") {
                    text.pop_front(1);
                }

                if text.is_empty() {
                    return LexerAction::Continue;
                }

                Token::Characters(SplitStatus::NotSplit, text)
            }
        };

        // tree-construction loop
        self.process_to_completion(token)
    }

    /// Finish lexer-driven parsing.
    fn end(&self) {
        for element in self.open_elements.borrow_mut().drain(..).rev() {
            self.on_open_element_removed(&element);
        }
    }

    /// Return whether the current adjusted node is foreign.
    fn adjusted_current_node_present_but_not_in_html_namespace(&self) -> bool {
        self.adjusted_current_node()
            .and_then(|current| self.open_element_name(&current))
            .is_some_and(|element_name| *element_name.ns() != ns!(html))
    }
}

impl Parser<'_> {
    /// Process one doctype token.
    fn process_doctype(&self, doctype: Doctype) -> LexerAction<Handle> {
        // wrong insertion mode
        if self.mode.get() != InsertionMode::Initial {
            self.builder.parse_error(if self.options.exact_errors {
                Cow::from(format!("DOCTYPE in insertion mode {:?}", self.mode.get()))
            } else {
                Cow::from("DOCTYPE in body")
            });

            return LexerAction::Continue;
        }

        let (is_bad_doctype, quirks_mode) =
            doctype::doctype_error_and_quirks(&doctype, self.options.iframe_srcdoc);

        // parse error
        if is_bad_doctype {
            self.builder.parse_error(if self.options.exact_errors {
                Cow::from(format!("Bad DOCTYPE: {doctype:?}"))
            } else {
                Cow::from("Bad DOCTYPE")
            });
        }

        let Doctype {
            name,
            public_id,
            system_id,
            force_quirks: _,
        } = doctype;

        // tree insertion
        if !self.options.drop_doctype {
            self.builder.append_doctype_to_document(
                name.unwrap_or(HtmlString::new()),
                public_id.unwrap_or(HtmlString::new()),
                system_id.unwrap_or(HtmlString::new()),
            );
        }

        // mode transition
        self.set_quirks_mode(quirks_mode);
        self.mode.set(InsertionMode::BeforeHtml);

        LexerAction::Continue
    }

    /// Step one parser token in one insertion mode.
    pub(crate) fn step(&self, mode: InsertionMode, token: Token) -> ProcessResult<Handle> {
        match mode {
            InsertionMode::InBody => self.step_body(token),
            InsertionMode::InTable
            | InsertionMode::InTableText
            | InsertionMode::InCaption
            | InsertionMode::InColumnGroup
            | InsertionMode::InTableBody
            | InsertionMode::InRow
            | InsertionMode::InCell => self.step_table(mode, token),
            _ => self.step_document(mode, token),
        }
    }

    /// Report one unexpected token for the current insertion mode.
    pub(super) fn unexpected<T: fmt::Debug>(&self, thing: &T) -> ProcessResult<Handle> {
        self.builder.parse_error(if self.options.exact_errors {
            Cow::from(format!(
                "Unexpected token {} in insertion mode {:?}",
                to_escaped_string(thing),
                self.mode.get()
            ))
        } else {
            Cow::from("Unexpected token")
        });

        ProcessResult::Done
    }

    /// Report when one popped element does not match the expected HTML name.
    pub(super) fn assert_named(&self, node: &Handle, name: LocalName) {
        // expected element
        if self.html_element_named(node, name) {
            return;
        }

        // parse error
        self.builder.parse_error(Cow::from(format!(
            "Unexpected open element while popping {name:?}"
        )));
    }

    /// Set the document quirks mode.
    pub(super) fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
        self.builder.set_quirks_mode(mode);
    }

    /// Finish parsing.
    pub(super) fn stop_parsing(&self) -> ProcessResult<Handle> {
        ProcessResult::Done
    }

    /// Switch the parser into text mode and tell the lexer which raw mode to use.
    pub(super) fn to_raw_text_mode(&self, kind: RawKind) -> ProcessResult<Handle> {
        self.orig_mode.set(Some(self.mode.get()));
        self.mode.set(InsertionMode::Text);
        ProcessResult::ToRawData(kind)
    }

    /// Insert one raw-text element and switch modes.
    pub(super) fn parse_raw_data(&self, tag: Tag, kind: RawKind) -> ProcessResult<Handle> {
        self.insert_element_for(tag);
        self.to_raw_text_mode(kind)
    }

    /// Return the body element when it is open in the standard slot.
    pub(super) fn body_element(&self) -> Option<Ref<'_, Handle>> {
        // missing body slot
        if self.open_elements.borrow().len() <= 1 {
            return None;
        }

        // body element
        let node = Ref::map(self.open_elements.borrow(), |elements| &elements[1]);
        if self.html_element_named(&node, local_name!("body")) {
            Some(node)
        } else {
            None
        }
    }

    /// Return whether the fragment context has one HTML local name.
    pub(super) fn fragment_context_named(&self, name: LocalName) -> bool {
        let context = self.context_element.borrow();

        context
            .as_ref()
            .is_some_and(|context| self.html_element_named(context, name))
    }

    /// Report one unexpected open element at the end of the body.
    pub(super) fn check_body_end(&self) {
        declare_tag_set!(body_end_ok =
            "dd" "dt" "li" "optgroup" "option" "p" "rp" "rt" "tbody" "td" "tfoot" "th"
            "thead" "tr" "body" "html");

        // unexpected trailing element
        for element in self.open_elements.borrow().iter() {
            let error = {
                let Some(element_name) = self.open_element_name(element) else {
                    return;
                };
                let name = element_name.expanded();
                if body_end_ok(name) {
                    continue;
                }

                if self.options.exact_errors {
                    Cow::from(format!("Unexpected open tag {name:?} at end of body"))
                } else {
                    Cow::from("Unexpected open tag at end of body")
                }
            };

            self.builder.parse_error(error);
            return;
        }
    }

    /// Reset the insertion mode from the open-element stack.
    pub(super) fn reset_insertion_mode(&self) -> InsertionMode {
        let open_elements = self.open_elements.borrow();

        // walk open elements from the current node outward
        for (index, mut node) in open_elements.iter().enumerate().rev() {
            let is_last = index == 0;
            let context_element = self.context_element.borrow();
            if let (true, Some(context)) = (is_last, context_element.as_ref()) {
                node = context;
            }

            // html namespace elements participate in insertion-mode reset
            let Some(element_name) = self.open_element_name(node) else {
                continue;
            };
            let local_name = match element_name.expanded() {
                ExpandedName {
                    ns: &ns!(html),
                    local,
                } => local,
                _ => continue,
            };

            // reset from the nearest qualifying element
            match *local_name {
                local_name!("td") | local_name!("th") => {
                    if !is_last {
                        return InsertionMode::InCell;
                    }
                }

                local_name!("tr") => return InsertionMode::InRow,

                local_name!("tbody") | local_name!("thead") | local_name!("tfoot") => {
                    return InsertionMode::InTableBody;
                }

                local_name!("caption") => return InsertionMode::InCaption,
                local_name!("colgroup") => return InsertionMode::InColumnGroup,
                local_name!("table") => return InsertionMode::InTable,

                local_name!("template") => {
                    let mode = self.template_modes.borrow().last().copied();
                    if let Some(mode) = mode {
                        return mode;
                    }

                    self.builder.parse_error(Borrowed(
                        "template mode missing while resetting insertion mode",
                    ));
                    return InsertionMode::InTemplate;
                }

                local_name!("head") => {
                    if !is_last {
                        return InsertionMode::InHead;
                    }
                }

                local_name!("body") => return InsertionMode::InBody,
                local_name!("frameset") => return InsertionMode::InFrameset,

                local_name!("html") => match *self.head_element.borrow() {
                    None => return InsertionMode::BeforeHead,
                    Some(_) => return InsertionMode::AfterHead,
                },

                _ => {}
            }
        }

        InsertionMode::InBody
    }
}
