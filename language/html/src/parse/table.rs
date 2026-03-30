use super::parser::{InlineEntry, InsertionMode, Parser, ProcessResult};
use super::tag::{declare_tag_set, tag, *};
use super::token::Token;
use crate::lex::{HtmlString, local_name};

use std::borrow::Cow::Borrowed;

/// Return whether one text payload contains non-whitespace characters.
fn any_not_whitespace(text: &HtmlString) -> bool {
    text.chars()
        .any(|character| !character.is_ascii_whitespace())
}

impl Parser<'_> {
    /// Process one table-related insertion mode step.
    pub(crate) fn step_table(
        &self,
        mode: InsertionMode,
        token: Token,
    ) -> ProcessResult<super::builder::Handle> {
        match mode {
            InsertionMode::InTable => self.process_in_table(token),
            InsertionMode::InTableText => self.process_in_table_text(token),
            InsertionMode::InCaption => self.process_in_caption(token),
            InsertionMode::InColumnGroup => self.process_in_column_group(token),
            InsertionMode::InTableBody => self.process_in_table_body(token),
            InsertionMode::InRow => self.process_in_row(token),
            InsertionMode::InCell => self.process_in_cell(token),
            _ => self.unexpected(&token),
        }
    }

    /// Process one token in the in-table insertion mode.
    fn process_in_table(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::NullCharacter | Token::Characters(..) => self.process_chars_in_table(token),
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag @ tag!(<caption>)) => {
                self.pop_until_current(table_scope);
                self.inline_elements.borrow_mut().push(InlineEntry::Marker);
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InCaption);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<colgroup>)) => {
                self.pop_until_current(table_scope);
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InColumnGroup);
                ProcessResult::Done
            }
            Token::Tag(tag!(<col>)) => {
                self.pop_until_current(table_scope);
                self.insert_phantom(local_name!("colgroup"));
                ProcessResult::Reprocess(InsertionMode::InColumnGroup, token)
            }
            Token::Tag(tag @ tag!(<tbody> | <tfoot> | <thead>)) => {
                self.pop_until_current(table_scope);
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InTableBody);
                ProcessResult::Done
            }
            Token::Tag(tag!(<td> | <th> | <tr>)) => {
                self.pop_until_current(table_scope);
                self.insert_phantom(local_name!("tbody"));
                ProcessResult::Reprocess(InsertionMode::InTableBody, token)
            }
            Token::Tag(tag!(<table>)) => {
                self.unexpected(&token);

                if self.in_scope_named(table_scope, local_name!("table")) {
                    self.pop_until_named(local_name!("table"));
                    ProcessResult::Reprocess(self.reset_insertion_mode(), token)
                } else {
                    ProcessResult::Done
                }
            }
            Token::Tag(tag!(</table>)) => {
                if self.in_scope_named(table_scope, local_name!("table")) {
                    self.pop_until_named(local_name!("table"));
                    self.mode.set(self.reset_insertion_mode());
                } else {
                    self.unexpected(&token);
                }

                ProcessResult::Done
            }
            Token::Tag(
                tag!(</body> | </caption> | </col> | </colgroup> | </html> |
                    </tbody> | </td> | </tfoot> | </th> | </thead> | </tr>),
            ) => self.unexpected(&token),
            Token::Tag(tag!(<style> | <script> | <template> | </template>)) => {
                self.step(InsertionMode::InHead, token)
            }
            Token::Tag(tag @ tag!(<input>)) => {
                self.unexpected(&tag);

                if self.is_type_hidden(&tag) {
                    self.insert_and_pop_element_for(tag);
                    ProcessResult::DoneAckSelfClosing
                } else {
                    self.foster_parent_in_body(Token::Tag(tag))
                }
            }
            Token::Tag(tag @ tag!(<form>)) => {
                self.unexpected(&tag);

                if !self.in_html_element_named(local_name!("template"))
                    && self.form_element.borrow().is_none()
                {
                    *self.form_element.borrow_mut() = Some(self.insert_and_pop_element_for(tag));
                }

                ProcessResult::Done
            }
            Token::Eof => self.step(InsertionMode::InBody, token),
            token => {
                self.unexpected(&token);
                self.foster_parent_in_body(token)
            }
        }
    }

    /// Process one token in the in-table-text insertion mode.
    fn process_in_table_text(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::NullCharacter => self.unexpected(&token),
            Token::Characters(split, text) => {
                self.pending_table_text.borrow_mut().push((split, text));
                ProcessResult::Done
            }
            token => {
                let pending = self.pending_table_text.take();
                let contains_nonspace = pending.iter().any(|&(split, ref text)| match split {
                    super::token::SplitStatus::Whitespace => false,
                    super::token::SplitStatus::NotWhitespace => true,
                    super::token::SplitStatus::NotSplit => any_not_whitespace(text),
                });

                if contains_nonspace {
                    self.builder.parse_error(Borrowed("Non-space table text"));

                    for (split, text) in pending {
                        let result = self.foster_parent_in_body(Token::Characters(split, text));

                        if !matches!(result, ProcessResult::Done) {
                            return result;
                        }
                    }
                } else {
                    for (_, text) in pending {
                        self.append_text(text);
                    }
                }

                match self.orig_mode.take() {
                    Some(mode) => ProcessResult::Reprocess(mode, token),
                    None => self.unexpected(&token),
                }
            }
        }
    }

    /// Process one token in the in-caption insertion mode.
    fn process_in_caption(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Tag(
                tag @ tag!(<caption> | <col> | <colgroup> | <tbody> | <td> | <tfoot> |
                    <th> | <thead> | <tr> | </table> | </caption>),
            ) => {
                if self.in_scope_named(table_scope, local_name!("caption")) {
                    self.generate_implied_end_tags(cursory_implied_end);
                    self.expect_to_close(local_name!("caption"));
                    self.clear_inline_elements_to_marker();

                    match tag {
                        crate::lex::Tag {
                            kind: crate::lex::TagKind::EndTag,
                            name: local_name!("caption"),
                            ..
                        } => {
                            self.mode.set(InsertionMode::InTable);
                            ProcessResult::Done
                        }
                        _ => ProcessResult::Reprocess(InsertionMode::InTable, Token::Tag(tag)),
                    }
                } else {
                    self.unexpected(&tag);
                    ProcessResult::Done
                }
            }
            Token::Tag(
                tag!(</body> | </col> | </colgroup> | </html> | </tbody> |
                    </td> | </tfoot> | </th> | </thead> | </tr>),
            ) => self.unexpected(&token),
            token => self.step(InsertionMode::InBody, token),
        }
    }

    /// Process one token in the in-column-group insertion mode.
    fn process_in_column_group(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Characters(super::token::SplitStatus::NotSplit, text) => {
                ProcessResult::SplitWhitespace(text)
            }
            Token::Characters(super::token::SplitStatus::Whitespace, text) => {
                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),
            Token::Tag(tag!(<html>)) => self.step(InsertionMode::InBody, token),
            Token::Tag(tag @ tag!(<col>)) => {
                self.insert_and_pop_element_for(tag);
                ProcessResult::DoneAckSelfClosing
            }
            Token::Tag(tag!(</colgroup>)) => {
                if self.current_node_named(local_name!("colgroup")) {
                    if self.pop().is_none() {
                        return ProcessResult::Done;
                    }
                    self.mode.set(InsertionMode::InTable);
                } else {
                    self.unexpected(&token);
                }

                ProcessResult::Done
            }
            Token::Tag(tag!(</col>)) => self.unexpected(&token),
            Token::Tag(tag!(<template> | </template>)) => self.step(InsertionMode::InHead, token),
            Token::Eof => self.step(InsertionMode::InBody, token),
            token => {
                if self.current_node_named(local_name!("colgroup")) {
                    if self.pop().is_none() {
                        ProcessResult::Done
                    } else {
                        ProcessResult::Reprocess(InsertionMode::InTable, token)
                    }
                } else {
                    self.unexpected(&token)
                }
            }
        }
    }

    /// Process one token in the in-table-body insertion mode.
    fn process_in_table_body(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Tag(tag @ tag!(<tr>)) => {
                self.pop_until_current(table_body_context);
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InRow);
                ProcessResult::Done
            }
            Token::Tag(tag!(<th> | <td>)) => {
                self.unexpected(&token);
                self.pop_until_current(table_body_context);
                self.insert_phantom(local_name!("tr"));
                ProcessResult::Reprocess(InsertionMode::InRow, token)
            }
            Token::Tag(tag @ tag!(</tbody> | </tfoot> | </thead>)) => {
                if self.in_scope_named(table_scope, tag.name) {
                    self.pop_until_current(table_body_context);
                    if self.pop().is_none() {
                        return ProcessResult::Done;
                    }
                    self.mode.set(InsertionMode::InTable);
                } else {
                    self.unexpected(&tag);
                }

                ProcessResult::Done
            }
            Token::Tag(
                tag!(<caption> | <col> | <colgroup> | <tbody> | <tfoot> | <thead> | </table>),
            ) => {
                declare_tag_set!(table_outer = "table" "tbody" "tfoot");

                if self.in_scope(table_scope, |element| {
                    self.element_in(&element, table_outer)
                }) {
                    self.pop_until_current(table_body_context);
                    if self.pop().is_none() {
                        ProcessResult::Done
                    } else {
                        ProcessResult::Reprocess(InsertionMode::InTable, token)
                    }
                } else {
                    self.unexpected(&token)
                }
            }
            Token::Tag(
                tag!(</body> | </caption> | </col> | </colgroup> | </html> | </td> | </th> | </tr>),
            ) => self.unexpected(&token),
            token => self.step(InsertionMode::InTable, token),
        }
    }

    /// Process one token in the in-row insertion mode.
    fn process_in_row(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Tag(tag @ tag!(<th> | <td>)) => {
                self.pop_until_current(table_row_context);
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InCell);
                self.inline_elements.borrow_mut().push(InlineEntry::Marker);
                ProcessResult::Done
            }
            Token::Tag(tag!(</tr>)) => {
                if self.in_scope_named(table_scope, local_name!("tr")) {
                    self.pop_until_current(table_row_context);
                    let Some(node) = self.pop() else {
                        return ProcessResult::Done;
                    };
                    self.assert_named(&node, local_name!("tr"));
                    self.mode.set(InsertionMode::InTableBody);
                } else {
                    self.unexpected(&token);
                }

                ProcessResult::Done
            }
            Token::Tag(
                tag!(<caption> | <col> | <colgroup> | <tbody> | <tfoot> | <thead> | <tr> | </table>),
            ) => {
                if self.in_scope_named(table_scope, local_name!("tr")) {
                    self.pop_until_current(table_row_context);
                    let Some(node) = self.pop() else {
                        return ProcessResult::Done;
                    };
                    self.assert_named(&node, local_name!("tr"));
                    ProcessResult::Reprocess(InsertionMode::InTableBody, token)
                } else {
                    self.unexpected(&token)
                }
            }
            Token::Tag(tag @ tag!(</tbody> | </tfoot> | </thead>)) => {
                if self.in_scope_named(table_scope, tag.name) {
                    if self.in_scope_named(table_scope, local_name!("tr")) {
                        self.pop_until_current(table_row_context);
                        let Some(node) = self.pop() else {
                            return ProcessResult::Done;
                        };
                        self.assert_named(&node, local_name!("tr"));
                        ProcessResult::Reprocess(InsertionMode::InTableBody, Token::Tag(tag))
                    } else {
                        ProcessResult::Done
                    }
                } else {
                    self.unexpected(&tag)
                }
            }
            Token::Tag(
                tag!(</body> | </caption> | </col> | </colgroup> | </html> | </td> | </th>),
            ) => self.unexpected(&token),
            token => self.step(InsertionMode::InTable, token),
        }
    }

    /// Process one token in the in-cell insertion mode.
    fn process_in_cell(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            Token::Tag(tag @ tag!(</td> | </th>)) => {
                if self.in_scope_named(table_scope, tag.name) {
                    self.generate_implied_end_tags(cursory_implied_end);
                    self.expect_to_close(tag.name);
                    self.clear_inline_elements_to_marker();
                    self.mode.set(InsertionMode::InRow);
                } else {
                    self.unexpected(&tag);
                }

                ProcessResult::Done
            }
            Token::Tag(
                tag!(<caption> | <col> | <colgroup> | <tbody> | <td> | <tfoot> | <th> | <thead> | <tr>),
            ) => {
                if self.in_scope(table_scope, |node| self.element_in(&node, td_th)) {
                    self.close_the_cell();
                    ProcessResult::Reprocess(InsertionMode::InRow, token)
                } else {
                    self.unexpected(&token)
                }
            }
            Token::Tag(tag!(</body> | </caption> | </col> | </colgroup> | </html>)) => {
                self.unexpected(&token)
            }
            Token::Tag(tag @ tag!(</table> | </tbody> | </tfoot> | </thead> | </tr>)) => {
                if self.in_scope_named(table_scope, tag.name) {
                    self.close_the_cell();
                    ProcessResult::Reprocess(InsertionMode::InRow, Token::Tag(tag))
                } else {
                    self.unexpected(&tag)
                }
            }
            token => self.step(InsertionMode::InBody, token),
        }
    }

    /// Handle one foster-parented in-body step.
    pub(super) fn foster_parent_in_body(
        &self,
        token: Token,
    ) -> ProcessResult<super::builder::Handle> {
        self.foster_parenting.set(true);
        let result = self.step(InsertionMode::InBody, token);
        self.foster_parenting.set(false);
        result
    }

    /// Handle one table character token.
    pub(super) fn process_chars_in_table(
        &self,
        token: Token,
    ) -> ProcessResult<super::builder::Handle> {
        declare_tag_set!(table_outer = "table" "tbody" "tfoot" "thead" "tr");

        if self.current_node_in(table_outer) {
            if !self.pending_table_text.borrow().is_empty() {
                self.builder
                    .parse_error(Borrowed("Pending table text was not flushed"));
                self.pending_table_text.borrow_mut().clear();
            }
            self.orig_mode.set(Some(self.mode.get()));
            ProcessResult::Reprocess(InsertionMode::InTableText, token)
        } else {
            self.builder
                .parse_error(Borrowed("Unexpected characters in table"));
            self.foster_parent_in_body(token)
        }
    }

    /// Close one open table cell.
    pub(super) fn close_the_cell(&self) {
        self.generate_implied_end_tags(cursory_implied_end);

        if self.pop_until(td_th) != 1 {
            self.builder
                .parse_error(Borrowed("expected to close <td> or <th> with cell"));
        }

        self.clear_inline_elements_to_marker();
    }
}
