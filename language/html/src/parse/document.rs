use super::insert::build_element_with_flags;
use super::parser::{InlineEntry, InsertionMode, Parser, ProcessResult};
use super::tag::{tag, *};
use super::token::Token;
use crate::lex::lexer::{Rawtext, Rcdata, ScriptData};
use crate::lex::{MetaEncodingScanner, QualifiedName, local_name, ns};

impl Parser<'_> {
    /// Reprocess one token after creating the root html element.
    fn fallback_before_html(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        self.create_root(vec![]);

        ProcessResult::Reprocess(InsertionMode::BeforeHead, token)
    }

    /// Reprocess one token after inserting one phantom head element.
    fn fallback_before_head(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        *self.head_element.borrow_mut() = Some(self.insert_phantom(local_name!("head")));

        ProcessResult::Reprocess(InsertionMode::InHead, token)
    }

    /// Reprocess one token after closing the head element.
    fn fallback_in_head(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        if self.pop().is_none() {
            return ProcessResult::Done;
        }

        ProcessResult::Reprocess(InsertionMode::AfterHead, token)
    }

    /// Reprocess one token after reporting one in-head-noscript parse error.
    fn fallback_in_head_noscript(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        self.unexpected(&token);

        if self.pop().is_none() {
            return ProcessResult::Done;
        }

        ProcessResult::Reprocess(InsertionMode::InHead, token)
    }

    /// Reprocess one token after inserting one phantom body element.
    fn fallback_after_head(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        self.insert_phantom(local_name!("body"));

        ProcessResult::Reprocess(InsertionMode::InBody, token)
    }

    /// Process one document-level insertion mode step.
    pub(crate) fn step_document(
        &self,
        mode: InsertionMode,
        token: Token,
    ) -> ProcessResult<super::builder::Handle> {
        match mode {
            InsertionMode::Initial => self.process_initial(token),
            InsertionMode::BeforeHtml => self.process_before_html(token),
            InsertionMode::BeforeHead => self.process_before_head(token),
            InsertionMode::InHead => self.process_in_head(token),
            InsertionMode::InHeadNoscript => self.process_in_head_noscript(token),
            InsertionMode::AfterHead => self.process_after_head(token),
            InsertionMode::Text => self.process_text(token),
            InsertionMode::InTemplate => self.process_in_template(token),
            InsertionMode::AfterBody => self.process_after_body(token),
            InsertionMode::InFrameset => self.process_in_frameset(token),
            InsertionMode::AfterFrameset => self.process_after_frameset(token),
            InsertionMode::AfterAfterBody => self.process_after_after_body(token),
            InsertionMode::AfterAfterFrameset => self.process_after_after_frameset(token),
            _ => self.unexpected(&token),
        }
    }

    /// Process one token in the initial insertion mode.
    fn process_initial(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => ProcessResult::Done,
            Token::Comment(text) => self.append_comment_to_doc(text),

            // anything else
            token => {
                if !self.options.iframe_srcdoc {
                    self.unexpected(&token);
                    self.set_quirks_mode(super::parser::Quirks);
                }

                ProcessResult::Reprocess(InsertionMode::BeforeHtml, token)
            }
        }
    }

    /// Process one token in the before-html insertion mode.
    fn process_before_html(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Comment(text) => self.append_comment_to_doc(text),
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => ProcessResult::Done,
            Token::Tag(tag @ tag!(<html>)) => {
                self.create_root(tag.attrs);
                self.mode.set(InsertionMode::BeforeHead);
                ProcessResult::Done
            }
            Token::Tag(tag!(</head> | </body> | </html> | </br>)) => {
                self.fallback_before_html(token)
            }
            Token::Tag(tag @ tag!(</>)) => self.unexpected(&tag),
            token => self.fallback_before_html(token),
        }
    }

    /// Process one token in the before-head insertion mode.
    fn process_before_head(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => ProcessResult::Done,
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Tag(tag @ tag!(<head>)) => {
                *self.head_element.borrow_mut() = Some(self.insert_element_for(tag));
                self.mode.set(InsertionMode::InHead);
                ProcessResult::Done
            }
            Token::Tag(tag!(</head> | </body> | </html> | </br>)) => {
                self.fallback_before_head(token)
            }
            Token::Tag(tag @ tag!(</>)) => self.unexpected(&tag),
            token => self.fallback_before_head(token),
        }
    }

    /// Process one token in the in-head insertion mode.
    fn process_in_head(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, text) => {
                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),

            // empty head metadata
            Token::Tag(tag @ tag!(<base> | <basefont> | <bgsound> | <link> | <meta>)) => {
                self.insert_and_pop_element_for(tag.clone());

                if let Some(charset) = tag.get_attribute(&local_name!("charset")) {
                    if let Some(charset) = MetaEncodingScanner::canonical_label(&charset) {
                        return ProcessResult::EncodingIndicator(charset);
                    }
                } else {
                    let has_content_type = match tag.get_attribute(&local_name!("http-equiv")) {
                        Some(value) => value.eq_ignore_ascii_case("content-type"),
                        None => false,
                    };

                    if has_content_type
                        && let Some(encoding) = tag
                            .get_attribute(&local_name!("content"))
                            .and_then(|content| {
                                MetaEncodingScanner::new_content(&content).scan_content_encoding()
                            })
                            .and_then(|encoding| MetaEncodingScanner::canonical_label(&encoding))
                    {
                        return ProcessResult::EncodingIndicator(encoding);
                    }
                }

                ProcessResult::DoneAckSelfClosing
            }

            // raw-text head content
            Token::Tag(tag @ tag!(<title>)) => self.parse_raw_data(tag, Rcdata),
            Token::Tag(tag @ tag!(<noframes> | <style> | <noscript>)) => {
                if !self.options.scripting_enabled && tag.name == local_name!("noscript") {
                    self.insert_element_for(tag);
                    self.mode.set(InsertionMode::InHeadNoscript);
                    ProcessResult::Done
                } else {
                    self.parse_raw_data(tag, Rawtext)
                }
            }

            // script handling
            Token::Tag(tag @ tag!(<script>)) => {
                let element = build_element_with_flags(
                    &self.builder,
                    QualifiedName::new(None, ns!(html), local_name!("script")),
                    tag.attrs,
                    tag.had_duplicate_attributes,
                );

                if self.is_fragment() {
                    self.builder.mark_script_already_started(&element);
                }

                self.insert_appropriately(super::parser::Child::Node(element), None);
                self.open_elements.borrow_mut().push(element);
                self.to_raw_text_mode(ScriptData)
            }

            // explicit head close
            Token::Tag(tag!(</head>)) => {
                if self.pop().is_none() {
                    return ProcessResult::Done;
                }
                self.mode.set(InsertionMode::AfterHead);
                ProcessResult::Done
            }

            // template
            Token::Tag(tag!(</body> | </html> | </br>)) => self.fallback_in_head(token),
            Token::Tag(tag @ tag!(<template>)) => {
                self.inline_elements.borrow_mut().push(InlineEntry::Marker);
                self.frameset_ok.set(false);
                self.mode.set(InsertionMode::InTemplate);
                self.template_modes
                    .borrow_mut()
                    .push(InsertionMode::InTemplate);

                if self.should_attach_declarative_shadow(&tag) {
                    let Some(mut shadow_host) = self.current_node() else {
                        return self.unexpected(&tag);
                    };

                    if self.is_fragment() && self.open_elements.borrow().len() == 1 {
                        let context = self.context_element.borrow();
                        let Some(context) = context.as_ref() else {
                            return self.unexpected(&tag);
                        };

                        shadow_host = *context;
                    }

                    let template = self.insert_foreign_element(tag.clone(), ns!(html), true);
                    let succeeded = self.attach_declarative_shadow(&tag, &shadow_host, &template);

                    if !succeeded {
                        if self.pop().is_none() {
                            return ProcessResult::Done;
                        }
                        self.insert_element_for(tag);
                    }
                } else {
                    self.insert_element_for(tag);
                }

                ProcessResult::Done
            }

            // template close
            Token::Tag(tag @ tag!(</template>)) => {
                if !self.in_html_element_named(local_name!("template")) {
                    self.unexpected(&tag);
                } else {
                    self.generate_implied_end_tags(thorough_implied_end);
                    self.expect_to_close(local_name!("template"));
                    self.clear_inline_elements_to_marker();
                    self.template_modes.borrow_mut().pop();
                    self.mode.set(self.reset_insertion_mode());
                }

                ProcessResult::Done
            }

            // parse error
            Token::Tag(tag!(<head> | </>)) => self.unexpected(&token),
            token => self.fallback_in_head(token),
        }
    }

    /// Process one token in the in-head-noscript insertion mode.
    fn process_in_head_noscript(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Tag(tag!(</noscript>)) => {
                if self.pop().is_none() {
                    return ProcessResult::Done;
                }
                self.mode.set(InsertionMode::InHead);
                ProcessResult::Done
            }
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => {
                self.step(InsertionMode::InHead, token)
            }
            Token::Comment(_) => self.step(InsertionMode::InHead, token),
            Token::Tag(tag!(<basefont> | <bgsound> | <link> | <meta> | <noframes> | <style>)) => {
                self.step(InsertionMode::InHead, token)
            }
            Token::Tag(tag!(</br>)) => self.fallback_in_head_noscript(token),
            Token::Tag(tag!(<head> | <noscript> | </>)) => self.unexpected(&token),
            token => self.fallback_in_head_noscript(token),
        }
    }

    /// Process one token in the after-head insertion mode.
    fn process_after_head(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, text) => {
                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),

            // body start
            Token::Tag(tag @ tag!(<body>)) => {
                self.insert_element_for(tag);
                self.frameset_ok.set(false);
                self.mode.set(InsertionMode::InBody);
                ProcessResult::Done
            }

            // frameset start
            Token::Tag(tag @ tag!(<frameset>)) => {
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InFrameset);
                ProcessResult::Done
            }

            // delayed head content
            Token::Tag(
                tag!(<base> | <basefont> | <bgsound> | <link> | <meta> |
                    <noframes> | <script> | <style> | <template> | <title>),
            ) => {
                self.unexpected(&token);
                let head = match self.head_element.borrow().as_ref().copied() {
                    Some(head) => head,
                    None => return self.unexpected(&token),
                };
                self.push(&head);
                let result = self.step(InsertionMode::InHead, token);
                self.remove_from_stack(&head);
                result
            }

            // head close and fallback
            Token::Tag(tag!(</template>)) => self.step(InsertionMode::InHead, token),
            Token::Tag(tag!(</body> | </html> | </br>)) => self.fallback_after_head(token),
            Token::Tag(tag!(<head> | </>)) => self.unexpected(&token),
            token => self.fallback_after_head(token),
        }
    }

    /// Process one token in the text insertion mode.
    fn process_text(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(_, text) => self.append_text(text),

            // eof in text mode
            Token::Eof => {
                self.unexpected(&token);

                if self.current_node_named(local_name!("script")) {
                    let open_elements = self.open_elements.borrow();
                    let Some(current) = open_elements.last() else {
                        return self.unexpected(&token);
                    };
                    self.builder.mark_script_already_started(current);
                }

                if self.pop().is_none() {
                    return ProcessResult::Done;
                }
                match self.orig_mode.take() {
                    Some(mode) => ProcessResult::Reprocess(mode, token),
                    None => self.unexpected(&token),
                }
            }
            Token::Tag(tag @ tag!(</>)) => {
                let Some(node) = self.pop() else {
                    return ProcessResult::Done;
                };
                let Some(mode) = self.orig_mode.take() else {
                    return self.unexpected(&tag);
                };
                self.mode.set(mode);

                if tag.name == local_name!("script") {
                    return ProcessResult::Script(node);
                }

                ProcessResult::Done
            }

            // any other token is invalid here
            token => self.unexpected(&token),
        }
    }

    /// Process one token in the in-template insertion mode.
    fn process_in_template(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(_, _) | Token::Comment(_) => self.step(InsertionMode::InBody, token),
            Token::Tag(
                tag!(<base> | <basefont> | <bgsound> | <link> | <meta> | <noframes> | <script> |
                    <style> | <template> | <title> | </template>),
            ) => self.step(InsertionMode::InHead, token),
            Token::Tag(tag!(<caption> | <colgroup> | <tbody> | <tfoot> | <thead>)) => {
                self.template_modes.borrow_mut().pop();
                self.template_modes
                    .borrow_mut()
                    .push(InsertionMode::InTable);
                ProcessResult::Reprocess(InsertionMode::InTable, token)
            }
            Token::Tag(tag!(<col>)) => {
                self.template_modes.borrow_mut().pop();
                self.template_modes
                    .borrow_mut()
                    .push(InsertionMode::InColumnGroup);
                ProcessResult::Reprocess(InsertionMode::InColumnGroup, token)
            }
            Token::Tag(tag!(<tr>)) => {
                self.template_modes.borrow_mut().pop();
                self.template_modes
                    .borrow_mut()
                    .push(InsertionMode::InTableBody);
                ProcessResult::Reprocess(InsertionMode::InTableBody, token)
            }
            Token::Tag(tag!(<td> | <th>)) => {
                self.template_modes.borrow_mut().pop();
                self.template_modes.borrow_mut().push(InsertionMode::InRow);
                ProcessResult::Reprocess(InsertionMode::InRow, token)
            }
            Token::Eof => {
                if !self.in_html_element_named(local_name!("template")) {
                    self.stop_parsing()
                } else {
                    self.unexpected(&token);
                    self.pop_until_named(local_name!("template"));
                    self.clear_inline_elements_to_marker();
                    self.template_modes.borrow_mut().pop();
                    self.mode.set(self.reset_insertion_mode());
                    ProcessResult::Reprocess(self.reset_insertion_mode(), token)
                }
            }
            Token::Tag(tag @ tag!(<>)) => {
                self.template_modes.borrow_mut().pop();
                self.template_modes.borrow_mut().push(InsertionMode::InBody);
                ProcessResult::Reprocess(InsertionMode::InBody, Token::Tag(tag))
            }
            token => self.unexpected(&token),
        }
    }

    /// Process one token in the after-body insertion mode.
    fn process_after_body(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => {
                self.step(InsertionMode::InBody, token)
            }
            Token::Comment(text) => self.append_comment_to_html(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Tag(tag!(</html>)) => {
                if self.is_fragment() {
                    self.unexpected(&token);
                } else {
                    self.mode.set(InsertionMode::AfterAfterBody);
                }

                ProcessResult::Done
            }
            Token::Eof => self.stop_parsing(),
            token => {
                self.unexpected(&token);
                ProcessResult::Reprocess(InsertionMode::InBody, token)
            }
        }
    }

    /// Process one token in the in-frameset insertion mode.
    fn process_in_frameset(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, text) => {
                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Tag(tag @ tag!(<frameset>)) => {
                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag!(</frameset>)) => {
                if self.open_elements.borrow().len() == 1 {
                    self.unexpected(&token);
                } else {
                    if self.pop().is_none() {
                        return ProcessResult::Done;
                    }

                    if !self.is_fragment() && !self.current_node_named(local_name!("frameset")) {
                        self.mode.set(InsertionMode::AfterFrameset);
                    }
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<frame>)) => {
                self.insert_and_pop_element_for(tag);
                ProcessResult::DoneAckSelfClosing
            }
            Token::Tag(tag!(<noframes>)) => self.step(InsertionMode::InHead, token),
            Token::Eof => {
                if self.open_elements.borrow().len() != 1 {
                    self.unexpected(&token);
                }

                self.stop_parsing()
            }
            token => self.unexpected(&token),
        }
    }

    /// Process one token in the after-frameset insertion mode.
    fn process_after_frameset(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, text) => {
                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Tag(tag!(</html>)) => {
                self.mode.set(InsertionMode::AfterAfterFrameset);
                ProcessResult::Done
            }
            Token::Tag(tag!(<noframes>)) => self.step(InsertionMode::InHead, token),
            Token::Eof => self.stop_parsing(),
            token => self.unexpected(&token),
        }
    }

    /// Process one token in the after-after-body insertion mode.
    fn process_after_after_body(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => {
                self.step(InsertionMode::InBody, token)
            }
            Token::Comment(text) => self.append_comment_to_doc(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Eof => self.stop_parsing(),
            token => {
                self.unexpected(&token);
                ProcessResult::Reprocess(InsertionMode::InBody, token)
            }
        }
    }

    /// Process one token in the after-after-frameset insertion mode.
    fn process_after_after_frameset(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, _) => {
                self.step(InsertionMode::InBody, token)
            }
            Token::Comment(text) => self.append_comment_to_doc(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Eof => self.stop_parsing(),
            Token::Tag(tag!(<noframes>)) => self.step(InsertionMode::InHead, token),
            token => self.unexpected(&token),
        }
    }
}
