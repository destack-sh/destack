use super::parser::{InlineEntry, InsertionMode, Parser, ProcessResult};
use super::tag::{declare_tag_set, tag, *};
use super::token::Token;
use crate::lex::TagKind::StartTag;
use crate::lex::lexer::{Rawtext, Rcdata};
use crate::lex::{HtmlString, LocalName, Tag, expanded_name, local_name, ns};

use std::borrow::Cow::Borrowed;

/// Return whether one text payload contains non-whitespace characters.
fn any_not_whitespace(text: &HtmlString) -> bool {
    text.chars()
        .any(|character| !character.is_ascii_whitespace())
}

/// Return whether one end tag name uses adoption-agency inline rules.
fn inline_end_tag_name(name: &LocalName) -> bool {
    matches!(
        name.to_owned_string().as_str(),
        "a" | "b"
            | "big"
            | "code"
            | "em"
            | "font"
            | "i"
            | "nobr"
            | "s"
            | "small"
            | "strike"
            | "strong"
            | "tt"
            | "u"
    )
}

/// Return whether one tag is one inline-style start tag.
fn is_inline_start_tag(tag: &Tag) -> bool {
    if !tag.is_start_tag() {
        return false;
    }

    matches!(
        tag.name.to_owned_string().as_str(),
        "b" | "big"
            | "code"
            | "em"
            | "font"
            | "i"
            | "s"
            | "small"
            | "strike"
            | "strong"
            | "tt"
            | "u"
    )
}

impl Parser<'_> {
    /// Process one in-body token.
    pub(crate) fn step_body(&self, token: Token) -> ProcessResult<super::builder::Handle> {
        match token {
            // text and comments
            Token::NullCharacter => self.unexpected(&token),
            Token::Characters(_, text) => {
                self.reconstruct_inline_elements();

                if any_not_whitespace(&text) {
                    self.frameset_ok.set(false);
                }

                self.append_text(text)
            }
            Token::Comment(text) => self.append_comment(text),

            // document structure
            Token::Tag(tag @ tag!(<html>)) => {
                self.unexpected(&tag);

                if !self.in_html_element_named(local_name!("template"))
                    && let Some(top) = self.html_root()
                {
                    self.builder.add_attrs_if_missing(&top, tag.attrs);
                }

                ProcessResult::Done
            }
            Token::Tag(
                tag!(<base> | <basefont> | <bgsound> | <link> | <meta> | <noframes>
                    | <script> | <style> | <template> | <title> | </template>),
            ) => self.step(InsertionMode::InHead, token),
            Token::Tag(tag @ tag!(<body>)) => {
                self.unexpected(&tag);
                let body_element = self.body_element().as_deref().cloned();

                match body_element {
                    Some(ref node)
                        if self.open_elements.borrow().len() != 1
                            && !self.in_html_element_named(local_name!("template")) =>
                    {
                        self.frameset_ok.set(false);
                        self.builder.add_attrs_if_missing(node, tag.attrs);
                    }
                    _ => {}
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<frameset>)) => {
                self.unexpected(&tag);

                if !self.frameset_ok.get() {
                    return ProcessResult::Done;
                }

                let Some(body) = self.body_element().map(|body| *body) else {
                    return ProcessResult::Done;
                };

                self.builder.remove_from_parent(&body);
                self.open_elements.borrow_mut().truncate(1);
                self.insert_element_for(tag);
                self.mode.set(InsertionMode::InFrameset);
                ProcessResult::Done
            }
            Token::Eof => {
                if !self.template_modes.borrow().is_empty() {
                    self.step(InsertionMode::InTemplate, token)
                } else {
                    self.check_body_end();
                    self.stop_parsing()
                }
            }
            Token::Tag(tag!(</body>)) => {
                if self.in_scope_named(default_scope, local_name!("body")) {
                    self.check_body_end();
                    self.mode.set(InsertionMode::AfterBody);
                } else {
                    self.builder
                        .parse_error(Borrowed("</body> with no <body> in scope"));
                }

                ProcessResult::Done
            }
            Token::Tag(tag!(</html>)) => {
                if self.in_scope_named(default_scope, local_name!("body")) {
                    self.check_body_end();
                    ProcessResult::Reprocess(InsertionMode::AfterBody, token)
                } else {
                    self.builder
                        .parse_error(Borrowed("</html> with no <body> in scope"));
                    ProcessResult::Done
                }
            }

            // sectioning and block structure
            Token::Tag(
                tag @
                tag!(<address> | <article> | <aside> | <blockquote> | <center> | <details> | <dialog> |
                    <dir> | <div> | <dl> | <fieldset> | <figcaption> | <figure> | <footer> | <header> |
                    <hgroup> | <main> | <nav> | <ol> | <p> | <search> | <section> | <summary> | <ul>),
            ) => {
                self.close_p_element_in_button_scope();
                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<menu>)) => {
                self.close_p_element_in_button_scope();
                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<h1> | <h2> | <h3> | <h4> | <h5> | <h6>)) => {
                self.close_p_element_in_button_scope();

                if self.current_node_in(heading_tag) {
                    self.builder.parse_error(Borrowed("nested heading tags"));

                    if self.pop().is_none() {
                        return ProcessResult::Done;
                    }
                }

                self.insert_element_for(tag);
                ProcessResult::Done
            }

            // preformatted and form controls
            Token::Tag(tag @ tag!(<pre> | <listing>)) => {
                self.close_p_element_in_button_scope();
                self.insert_element_for(tag);
                self.ignore_lf.set(true);
                self.frameset_ok.set(false);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<form>)) => {
                if self.form_element.borrow().is_some()
                    && !self.in_html_element_named(local_name!("template"))
                {
                    self.builder.parse_error(Borrowed("nested forms"));
                } else {
                    self.close_p_element_in_button_scope();
                    let element = self.insert_element_for(tag);

                    if !self.in_html_element_named(local_name!("template")) {
                        *self.form_element.borrow_mut() = Some(element);
                    }
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<li> | <dd> | <dt>)) => {
                declare_tag_set!(close_list = "li");
                declare_tag_set!(close_defn = "dd" "dt");
                declare_tag_set!(extra_special = [special_tag] - "address" "div" "p");
                let is_list = tag.name == local_name!("li");

                self.frameset_ok.set(false);

                let mut to_close = None;
                for node in self.open_elements.borrow().iter().rev() {
                    let Some(element_name) = self.open_element_name(node) else {
                        break;
                    };
                    let name = element_name.expanded();
                    let can_close = if is_list {
                        close_list(name)
                    } else {
                        close_defn(name)
                    };

                    if can_close {
                        to_close = Some(*name.local);
                        break;
                    }

                    if extra_special(name) {
                        break;
                    }
                }

                if let Some(name) = to_close {
                    self.generate_implied_end_except(name);
                    self.expect_to_close(name);
                }

                self.close_p_element_in_button_scope();
                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<plaintext>)) => {
                self.close_p_element_in_button_scope();
                self.insert_element_for(tag);
                ProcessResult::ToPlaintext
            }
            Token::Tag(tag @ tag!(<button>)) => {
                if self.in_scope_named(default_scope, local_name!("button")) {
                    self.builder.parse_error(Borrowed("nested buttons"));
                    self.generate_implied_end_tags(cursory_implied_end);
                    self.pop_until_named(local_name!("button"));
                }

                self.reconstruct_inline_elements();
                self.insert_element_for(tag);
                self.frameset_ok.set(false);
                ProcessResult::Done
            }

            // block end tags
            Token::Tag(
                tag
                @ tag!(</address> | </article> | </aside> | </blockquote> | </button> | </center> |
                    </details> | </dialog> | </dir> | </div> | </dl> | </fieldset> | </figcaption> |
                    </figure> | </footer> | </header> | </hgroup> | </listing> | </main> | </menu> |
                    </nav> | </ol> | </pre> | </search> | </section> | </select> | </summary> | </ul>),
            ) => {
                if !self.in_scope_named(default_scope, tag.name) {
                    self.unexpected(&tag);
                } else {
                    self.generate_implied_end_tags(cursory_implied_end);
                    self.expect_to_close(tag.name);
                }

                ProcessResult::Done
            }
            Token::Tag(tag!(</form>)) => {
                if !self.in_html_element_named(local_name!("template")) {
                    let Some(node) = self.form_element.take() else {
                        self.builder
                            .parse_error(Borrowed("Null form element pointer on </form>"));
                        return ProcessResult::Done;
                    };

                    if !self.in_scope(default_scope, |element| {
                        self.builder.same_node(&node, &element)
                    }) {
                        self.builder
                            .parse_error(Borrowed("Form element not in scope on </form>"));
                        return ProcessResult::Done;
                    }

                    self.generate_implied_end_tags(cursory_implied_end);
                    let Some(current) = self.current_node() else {
                        self.builder
                            .parse_error(Borrowed("Missing current open element on </form>"));
                        return ProcessResult::Done;
                    };
                    self.remove_from_stack(&node);

                    if !self.builder.same_node(&current, &node) {
                        self.builder
                            .parse_error(Borrowed("Bad open element on </form>"));
                    }
                } else {
                    if !self.in_scope_named(default_scope, local_name!("form")) {
                        self.builder
                            .parse_error(Borrowed("Form element not in scope on </form>"));
                        return ProcessResult::Done;
                    }

                    self.generate_implied_end_tags(cursory_implied_end);

                    if !self.current_node_named(local_name!("form")) {
                        self.builder
                            .parse_error(Borrowed("Bad open element on </form>"));
                    }

                    self.pop_until_named(local_name!("form"));
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(</option>)) => {
                self.process_end_tag_in_body(tag);
                ProcessResult::Done
            }
            Token::Tag(tag!(</p>)) => {
                if !self.in_scope_named(button_scope, local_name!("p")) {
                    self.builder.parse_error(Borrowed("No <p> tag to close"));
                    self.insert_phantom(local_name!("p"));
                }

                self.close_p_element();
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(</li> | </dd> | </dt>)) => {
                let in_scope = if tag.name == local_name!("li") {
                    self.in_scope_named(list_item_scope, tag.name)
                } else {
                    self.in_scope_named(default_scope, tag.name)
                };

                if in_scope {
                    self.generate_implied_end_except(tag.name);
                    self.expect_to_close(tag.name);
                } else {
                    self.builder
                        .parse_error(Borrowed("No matching tag to close"));
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(</h1> | </h2> | </h3> | </h4> | </h5> | </h6>)) => {
                if self.in_scope(default_scope, |element| {
                    self.element_in(&element, heading_tag)
                }) {
                    self.generate_implied_end_tags(cursory_implied_end);

                    if !self.current_node_named(tag.name) {
                        self.builder
                            .parse_error(Borrowed("Closing wrong heading tag"));
                    }

                    self.pop_until(heading_tag);
                } else {
                    self.builder
                        .parse_error(Borrowed("No heading tag to close"));
                }

                ProcessResult::Done
            }
            Token::Tag(tag) if tag.is_start_tag_named(&local_name!("a")) => {
                self.handle_misnested_a_tags(&tag);
                self.reconstruct_inline_elements();
                self.insert_inline_element(tag);
                ProcessResult::Done
            }
            Token::Tag(tag) if is_inline_start_tag(&tag) => {
                self.reconstruct_inline_elements();
                self.insert_inline_element(tag);
                ProcessResult::Done
            }
            Token::Tag(tag) if tag.is_start_tag_named(&local_name!("nobr")) => {
                self.reconstruct_inline_elements();

                if self.in_scope_named(default_scope, local_name!("nobr")) {
                    self.builder.parse_error(Borrowed("Nested <nobr>"));
                    self.adoption_agency(local_name!("nobr"));
                    self.reconstruct_inline_elements();
                }

                self.insert_inline_element(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(</>)) if inline_end_tag_name(&tag.name) => {
                self.adoption_agency(tag.name);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<applet> | <marquee> | <object>)) => {
                self.reconstruct_inline_elements();
                self.insert_element_for(tag);
                self.inline_elements.borrow_mut().push(InlineEntry::Marker);
                self.frameset_ok.set(false);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(</applet> | </marquee> | </object>)) => {
                if !self.in_scope_named(default_scope, tag.name) {
                    self.unexpected(&tag);
                } else {
                    self.generate_implied_end_tags(cursory_implied_end);
                    self.expect_to_close(tag.name);
                    self.clear_inline_elements_to_marker();
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<table>)) => {
                if self.quirks_mode.get() != super::parser::Quirks {
                    self.close_p_element_in_button_scope();
                }

                self.insert_element_for(tag);
                self.frameset_ok.set(false);
                self.mode.set(InsertionMode::InTable);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(</br>)) => {
                self.unexpected(&tag);
                self.step(
                    InsertionMode::InBody,
                    Token::Tag(Tag {
                        kind: StartTag,
                        attrs: vec![],
                        ..tag
                    }),
                )
            }
            Token::Tag(tag @ tag!(<area> | <br> | <embed> | <img> | <keygen> | <wbr>)) => {
                self.reconstruct_inline_elements();
                self.insert_and_pop_element_for(tag);
                self.frameset_ok.set(false);
                ProcessResult::DoneAckSelfClosing
            }
            Token::Tag(tag @ tag!(<input>)) => {
                if self.is_fragment() && self.fragment_context_named(local_name!("select")) {
                    self.unexpected(&tag);
                }

                if self.in_scope_named(default_scope, local_name!("select")) {
                    self.unexpected(&tag);
                    self.pop_until_named(local_name!("select"));
                }

                let is_type_hidden = self.is_type_hidden(&tag);

                self.reconstruct_inline_elements();
                self.insert_and_pop_element_for(tag);

                if !is_type_hidden {
                    self.frameset_ok.set(false);
                }

                ProcessResult::DoneAckSelfClosing
            }
            Token::Tag(tag @ tag!(<param> | <source> | <track>)) => {
                self.insert_and_pop_element_for(tag);
                ProcessResult::DoneAckSelfClosing
            }
            Token::Tag(tag @ tag!(<hr>)) => {
                self.close_p_element_in_button_scope();

                if self.in_scope_named(default_scope, local_name!("select")) {
                    self.generate_implied_end_tags(cursory_implied_end);

                    if self.in_scope_named(default_scope, local_name!("option"))
                        || self.in_scope_named(default_scope, local_name!("optgroup"))
                    {
                        self.builder.parse_error(Borrowed("hr in option"));
                    }
                }

                self.insert_and_pop_element_for(tag);
                self.frameset_ok.set(false);
                ProcessResult::DoneAckSelfClosing
            }
            Token::Tag(tag @ tag!(<image>)) => {
                self.unexpected(&tag);
                self.step(
                    InsertionMode::InBody,
                    Token::Tag(Tag {
                        name: local_name!("img"),
                        ..tag
                    }),
                )
            }
            Token::Tag(tag @ tag!(<textarea>)) => {
                self.ignore_lf.set(true);
                self.frameset_ok.set(false);
                self.parse_raw_data(tag, Rcdata)
            }
            Token::Tag(tag @ tag!(<xmp>)) => {
                self.close_p_element_in_button_scope();
                self.reconstruct_inline_elements();
                self.frameset_ok.set(false);
                self.parse_raw_data(tag, Rawtext)
            }
            Token::Tag(tag @ tag!(<iframe>)) => {
                self.frameset_ok.set(false);
                self.parse_raw_data(tag, Rawtext)
            }
            Token::Tag(tag @ tag!(<noembed>)) => self.parse_raw_data(tag, Rawtext),
            Token::Tag(tag @ tag!(<select>)) => {
                if self.is_fragment() && self.fragment_context_named(local_name!("select")) {
                    self.unexpected(&tag);
                } else if self.in_scope_named(default_scope, local_name!("select")) {
                    self.unexpected(&tag);
                    self.pop_until_named(local_name!("select"));
                } else {
                    self.reconstruct_inline_elements();
                    self.insert_element_for(tag);
                    self.frameset_ok.set(false);
                }

                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<option>)) => {
                if self.in_scope_named(default_scope, local_name!("select")) {
                    self.generate_implied_end_except(local_name!("optgroup"));

                    if self.in_scope_named(default_scope, local_name!("option")) {
                        self.builder.parse_error(Borrowed("nested options"));
                    }
                } else if self.current_node_named(local_name!("option")) && self.pop().is_none() {
                    return ProcessResult::Done;
                }

                self.reconstruct_inline_elements();
                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<optgroup>)) => {
                if self.in_scope_named(default_scope, local_name!("select")) {
                    self.generate_implied_end_tags(cursory_implied_end);

                    if self.in_scope_named(default_scope, local_name!("option"))
                        || self.in_scope_named(default_scope, local_name!("optgroup"))
                    {
                        self.builder.parse_error(Borrowed("nested options"));
                    }
                } else if self.current_node_named(local_name!("option")) && self.pop().is_none() {
                    return ProcessResult::Done;
                }

                self.reconstruct_inline_elements();
                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<rb> | <rtc>)) => {
                if self.in_scope_named(default_scope, local_name!("ruby")) {
                    self.generate_implied_end_tags(cursory_implied_end);
                }

                if !self.current_node_named(local_name!("ruby")) {
                    self.unexpected(&tag);
                }

                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<rp> | <rt>)) => {
                if self.in_scope_named(default_scope, local_name!("ruby")) {
                    self.generate_implied_end_except(local_name!("rtc"));
                }

                if !self.current_node_named(local_name!("rtc"))
                    && !self.current_node_named(local_name!("ruby"))
                {
                    self.unexpected(&tag);
                }

                self.insert_element_for(tag);
                ProcessResult::Done
            }
            Token::Tag(tag) if tag.is_start_tag_named(&local_name!("math")) => {
                self.reconstruct_inline_elements();
                self.enter_foreign(tag, ns!(mathml))
            }
            Token::Tag(tag) if tag.is_start_tag_named(&local_name!("svg")) => {
                self.reconstruct_inline_elements();
                self.enter_foreign(tag, ns!(svg))
            }
            Token::Tag(
                tag!(<caption> | <col> | <colgroup> | <frame> | <head> |
                    <tbody> | <td> | <tfoot> | <th> | <thead> | <tr>),
            ) => {
                self.unexpected(&token);
                ProcessResult::Done
            }
            Token::Tag(tag @ tag!(<>)) => {
                if self.options.scripting_enabled && tag.name == local_name!("noscript") {
                    self.parse_raw_data(tag, Rawtext)
                } else {
                    self.reconstruct_inline_elements();
                    self.insert_element_for(tag);
                    ProcessResult::Done
                }
            }
            Token::Tag(tag @ tag!(</>)) => {
                self.process_end_tag_in_body(tag);
                ProcessResult::Done
            }
        }
    }

    /// Return whether one input tag is hidden.
    pub(super) fn is_type_hidden(&self, tag: &Tag) -> bool {
        match tag
            .attrs
            .iter()
            .find(|attribute| attribute.name.expanded() == expanded_name!("", "type"))
        {
            None => false,
            Some(attribute) => attribute.value.eq_ignore_ascii_case("hidden"),
        }
    }
}
