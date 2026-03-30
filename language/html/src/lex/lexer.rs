use super::token::{Doctype, LexHandler, LexerResult, StartTag, TagKind};

use super::buffer::BufferQueue;
use super::macros::time;
use super::reference::CharacterReferenceTokenizer;
use super::{Attribute, HtmlString, LocalName};

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

pub(crate) use self::LexerMode::*;
pub(crate) use self::RawKind::*;

/// One script-escape substate inside script raw text.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Debug)]
pub(crate) enum ScriptEscapeKind {
    /// The normal escaped script state.
    Escaped,
    /// The double-escaped script state.
    DoubleEscaped,
}

/// One doctype identifier slot.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Debug)]
pub(crate) enum DoctypeIdKind {
    /// The public identifier slot.
    Public,
    /// The system identifier slot.
    System,
}

/// One raw-text family handled by the lexer.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Debug)]
pub(crate) enum RawKind {
    /// The RCDATA family.
    Rcdata,
    /// The RAWTEXT family.
    Rawtext,
    /// The `<script>` data family.
    ScriptData,
    /// One escaped `<script>` family.
    ScriptDataEscaped(ScriptEscapeKind),
}

/// One attribute-value quoting mode.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Debug)]
pub(crate) enum AttrValueKind {
    /// One unquoted attribute value.
    Unquoted,
    /// One single-quoted attribute value.
    SingleQuoted,
    /// One double-quoted attribute value.
    DoubleQuoted,
}

/// One HTML lexer state.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Debug)]
pub(crate) enum LexerMode {
    /// The normal data state.
    ///
    /// See <https://html.spec.whatwg.org/#data-state>.
    Data,
    /// The plaintext state.
    ///
    /// See <https://html.spec.whatwg.org/#plaintext-state>.
    Plaintext,
    /// The tag-open state.
    ///
    /// See <https://html.spec.whatwg.org/#tag-open-state>.
    TagOpen,
    /// The end-tag-open state.
    ///
    /// See <https://html.spec.whatwg.org/#end-tag-open-state>.
    EndTagOpen,
    /// The tag-name state.
    ///
    /// See <https://html.spec.whatwg.org/#tag-name-state>.
    TagName,
    /// One raw-text data state.
    RawData(RawKind),
    /// One raw-text less-than-sign state.
    RawLessThanSign(RawKind),
    /// One raw-text end-tag-open state.
    RawEndTagOpen(RawKind),
    /// One raw-text end-tag-name state.
    RawEndTagName(RawKind),
    /// One script-data-escape-start state.
    ScriptDataEscapeStart(ScriptEscapeKind),
    /// The script-data-escape-start-dash state.
    ///
    /// See <https://html.spec.whatwg.org/#script-data-escape-start-dash-state>.
    ScriptDataEscapeStartDash,
    /// One script-data-escaped-dash state.
    ScriptDataEscapedDash(ScriptEscapeKind),
    /// One script-data-escaped-dash-dash state.
    ScriptDataEscapedDashDash(ScriptEscapeKind),
    /// The script-data-double-escape-end state.
    ///
    /// See <https://html.spec.whatwg.org/#script-data-double-escape-end-state>.
    ScriptDataDoubleEscapeEnd,
    /// The before-attribute-name state.
    ///
    /// See <https://html.spec.whatwg.org/#before-attribute-name-state>.
    BeforeAttributeName,
    /// The attribute-name state.
    ///
    /// See <https://html.spec.whatwg.org/#attribute-name-state>.
    AttributeName,
    /// The after-attribute-name state.
    ///
    /// See <https://html.spec.whatwg.org/#after-attribute-name-state>.
    AfterAttributeName,
    /// The before-attribute-value state.
    ///
    /// See <https://html.spec.whatwg.org/#before-attribute-value-state>.
    BeforeAttributeValue,
    /// One attribute-value state with one quoting mode.
    AttributeValue(AttrValueKind),
    /// The after-attribute-value-quoted state.
    ///
    /// See <https://html.spec.whatwg.org/#after-attribute-value-(quoted)-state>.
    AfterAttributeValueQuoted,
    /// The self-closing-start-tag state.
    ///
    /// See <https://html.spec.whatwg.org/#self-closing-start-tag-state>.
    SelfClosingStartTag,
    /// The bogus-comment state.
    ///
    /// See <https://html.spec.whatwg.org/#bogus-comment-state>.
    BogusComment,
    /// The markup-declaration-open state.
    ///
    /// See <https://html.spec.whatwg.org/#markup-declaration-open-state>.
    MarkupDeclarationOpen,
    /// The comment-start state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-start-state>.
    CommentStart,
    /// The comment-start-dash state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-start-dash-state>.
    CommentStartDash,
    /// The comment state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-state>.
    Comment,
    /// The comment-less-than-sign state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-less-than-sign-state>.
    CommentLessThanSign,
    /// The comment-less-than-sign-bang state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-less-than-sign-bang-state>.
    CommentLessThanSignBang,
    /// The comment-less-than-sign-bang-dash state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-less-than-sign-bang-dash-state>.
    CommentLessThanSignBangDash,
    /// The comment-less-than-sign-bang-dash-dash state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-less-than-sign-bang-dash-dash-state>.
    CommentLessThanSignBangDashDash,
    /// The comment-end-dash state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-end-dash-state>.
    CommentEndDash,
    /// The comment-end state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-end-state>.
    CommentEnd,
    /// The comment-end-bang state.
    ///
    /// See <https://html.spec.whatwg.org/#comment-end-bang-state>.
    CommentEndBang,
    /// The doctype state.
    ///
    /// See <https://html.spec.whatwg.org/#doctype-state>.
    Doctype,
    /// The before-doctype-name state.
    ///
    /// See <https://html.spec.whatwg.org/#before-doctype-name-state>.
    BeforeDoctypeName,
    /// The doctype-name state.
    ///
    /// See <https://html.spec.whatwg.org/#doctype-name-state>.
    DoctypeName,
    /// The after-doctype-name state.
    ///
    /// See <https://html.spec.whatwg.org/#after-doctype-name-state>.
    AfterDoctypeName,
    /// One after-doctype-keyword state.
    AfterDoctypeKeyword(DoctypeIdKind),
    /// One before-doctype-identifier state.
    BeforeDoctypeIdentifier(DoctypeIdKind),
    /// One double-quoted doctype-identifier state.
    DoctypeIdentifierDoubleQuoted(DoctypeIdKind),
    /// One single-quoted doctype-identifier state.
    DoctypeIdentifierSingleQuoted(DoctypeIdKind),
    /// One after-doctype-identifier state.
    AfterDoctypeIdentifier(DoctypeIdKind),
    /// The between-doctype-public-and-system-identifiers state.
    ///
    /// See <https://html.spec.whatwg.org/#between-doctype-public-and-system-identifiers-state>.
    BetweenDoctypePublicAndSystemIdentifiers,
    /// The bogus-doctype state.
    ///
    /// See <https://html.spec.whatwg.org/#bogus-doctype-state>.
    BogusDoctype,
    /// The CDATA-section state.
    ///
    /// See <https://html.spec.whatwg.org/#cdata-section-state>.
    CdataSection,
    /// The CDATA-section-bracket state.
    ///
    /// See <https://html.spec.whatwg.org/#cdata-section-bracket-state>.
    CdataSectionBracket,
    /// The CDATA-section-end state.
    ///
    /// See <https://html.spec.whatwg.org/#cdata-section-end-state>.
    CdataSectionEnd,
}

/// The result of invoking the tokenizer once.
pub(crate) enum ProcessResult<Handle> {
    /// The tokenizer should be re-invoked immediately.
    Continue,
    /// The tokenizer has not finished, but it needs to wait for more
    /// input to arrive before it can continue.
    Suspend,
    /// The tokenizer was blocked by a `<script>`.
    ///
    /// This `<script>` needs to be executed before tokenization
    /// can continue, as it might invoke `document.write`.
    Script(Handle),
    /// The tokenizer was blocked because it found a `<meta charset>` tag.
    ///
    /// Such tags may force the user agent to re-parse the document with the new
    /// encoding, but non-conformant implementations can reasonably treat
    /// this as [Self::Continue].
    EncodingIndicator(HtmlString),
}

/// Lexer options, with an impl for `Default`.
#[derive(Debug, Clone)]
pub(crate) struct LexerOptions {
    /// Report all parse errors described in the spec, at some performance cost.
    pub(crate) exact_errors: bool,

    /// Discard a leading `U+FEFF BYTE ORDER MARK`.
    pub(crate) discard_bom: bool,

    /// Keep per-state timing data for debugging output.
    pub(crate) profile: bool,

    /// The initial state override.
    ///
    /// Only the test runner should set a value here.
    pub(crate) initial_state: Option<LexerMode>,

    /// The last start tag name override.
    ///
    /// Only the test runner should set a value here.
    /// The string form keeps lexer options sendable.
    pub(crate) last_start_tag_name: Option<String>,
}

impl Default for LexerOptions {
    /// Create the default lexer options.
    fn default() -> LexerOptions {
        LexerOptions {
            exact_errors: false,
            discard_bom: true,
            profile: false,
            initial_state: None,
            last_start_tag_name: None,
        }
    }
}

/// The HTML lexer.
#[derive(Debug)]
pub(crate) struct Lexer<Parser> {
    /// Options controlling the behavior of the lexer.
    pub(super) options: LexerOptions,

    /// The parser receiving lexer output.
    pub(crate) parser: Parser,

    /// The abstract machine state as described in the spec.
    pub(super) state: Cell<LexerMode>,

    /// Whether the lexer has reached the end of the file once queued buffers drain.
    pub(super) at_eof: Cell<bool>,

    /// The nested character-reference lexer when one is active.
    pub(super) character_reference_tokenizer: RefCell<Option<CharacterReferenceTokenizer>>,

    /// The current input character.
    pub(super) current_char: Cell<char>,

    /// Whether to reconsume the current input character.
    pub(super) reconsume: Cell<bool>,

    /// Whether a translated carriage return should suppress the next line feed.
    pub(super) ignore_lf: Cell<bool>,

    /// Whether to discard a leading `U+FEFF BYTE ORDER MARK`.
    pub(super) discard_bom: Cell<bool>,

    /// Current tag kind.
    pub(super) current_tag_kind: Cell<TagKind>,

    /// Current tag name.
    pub(super) current_tag_name: RefCell<HtmlString>,

    /// Whether the current tag is self-closing.
    pub(super) current_tag_self_closing: Cell<bool>,

    /// Whether the current tag had duplicate attributes.
    pub(super) current_tag_had_duplicate_attributes: Cell<bool>,

    /// Current tag attributes.
    pub(super) current_tag_attrs: RefCell<Vec<Attribute>>,

    /// Current attribute name.
    pub(super) current_attr_name: RefCell<HtmlString>,

    /// Current attribute value.
    pub(super) current_attr_value: RefCell<HtmlString>,

    /// Current comment.
    pub(super) current_comment: RefCell<HtmlString>,

    /// Current doctype token.
    pub(super) current_doctype: RefCell<Doctype>,

    /// The last start tag name for appropriate-end-tag checks.
    pub(super) last_start_tag_name: RefCell<Option<LocalName>>,

    /// The temporary buffer mentioned in the spec.
    pub(super) temp_buf: RefCell<HtmlString>,

    /// Record of how many ns we spent in each state, if profiling is enabled.
    pub(super) state_profile: RefCell<BTreeMap<LexerMode, u64>>,

    /// Record of how many ns we spent in the parser callbacks.
    pub(super) time_in_parser: Cell<u64>,

    /// The current input line.
    pub(super) current_line: Cell<u64>,
}

#[allow(clippy::clone_on_copy)]
impl<Parser: LexHandler> Lexer<Parser> {
    /// Create a new lexer which feeds tokens to one parser.
    pub(crate) fn new(parser: Parser, mut options: LexerOptions) -> Lexer<Parser> {
        let start_tag_name = options
            .last_start_tag_name
            .take()
            .map(|s| LocalName::from(&*s));
        let state = options.initial_state.unwrap_or(Data);
        let discard_bom = options.discard_bom;
        Lexer {
            options,
            parser,
            state: Cell::new(state),
            character_reference_tokenizer: RefCell::new(None),
            at_eof: Cell::new(false),
            current_char: Cell::new('\0'),
            reconsume: Cell::new(false),
            ignore_lf: Cell::new(false),
            discard_bom: Cell::new(discard_bom),
            current_tag_kind: Cell::new(StartTag),
            current_tag_name: RefCell::new(HtmlString::new()),
            current_tag_self_closing: Cell::new(false),
            current_tag_had_duplicate_attributes: Cell::new(false),
            current_tag_attrs: RefCell::new(vec![]),
            current_attr_name: RefCell::new(HtmlString::new()),
            current_attr_value: RefCell::new(HtmlString::new()),
            current_comment: RefCell::new(HtmlString::new()),
            current_doctype: RefCell::new(Doctype::default()),
            last_start_tag_name: RefCell::new(start_tag_name),
            temp_buf: RefCell::new(HtmlString::new()),
            state_profile: RefCell::new(BTreeMap::new()),
            time_in_parser: Cell::new(0),
            current_line: Cell::new(1),
        }
    }

    /// Feed an input string into the lexer.
    pub(crate) fn feed(&self, input: &BufferQueue) -> LexerResult<Parser::Handle> {
        if input.is_empty() {
            return LexerResult::Done;
        }

        if self.discard_bom.get() {
            if let Some(c) = input.peek() {
                if c == '\u{feff}' {
                    input.next();
                }
            } else {
                return LexerResult::Done;
            }
        };

        self.run(input)
    }

    /// Run the state machine for as long as we can.
    pub(super) fn run(&self, input: &BufferQueue) -> LexerResult<Parser::Handle> {
        if self.options.profile {
            loop {
                let state = self.state.get();
                let old_parser = self.time_in_parser.get();
                let (run, mut elapsed_ns) = time!(self.step(input));
                elapsed_ns -= self.time_in_parser.get() - old_parser;
                let new = match self.state_profile.borrow_mut().get_mut(&state) {
                    Some(x) => {
                        *x += elapsed_ns;
                        false
                    }
                    None => true,
                };
                if new {
                    // do this here because of borrow shenanigans
                    self.state_profile.borrow_mut().insert(state, elapsed_ns);
                }
                match run {
                    ProcessResult::Continue => (),
                    ProcessResult::Suspend => break,
                    ProcessResult::Script(node) => return LexerResult::Script(node),
                    ProcessResult::EncodingIndicator(encoding) => {
                        return LexerResult::EncodingIndicator(encoding);
                    }
                }
            }
        } else {
            loop {
                match self.step(input) {
                    ProcessResult::Continue => (),
                    ProcessResult::Suspend => break,
                    ProcessResult::Script(node) => return LexerResult::Script(node),
                    ProcessResult::EncodingIndicator(encoding) => {
                        return LexerResult::EncodingIndicator(encoding);
                    }
                }
            }
        }

        LexerResult::Done
    }
}
