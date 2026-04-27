use crate::lex::buffer::BufferQueue;
use crate::lex::lexer::LexerMode::{self, Data, Plaintext, RawData};
use crate::lex::{
    HtmlString, Lexer, LexerOptions, LexerResult, LocalName, Namespace, Prefix, QualifiedName,
    RawKind, Tag, expanded_name, namespace_prefix, ns,
};
use crate::parse::parser::RawKind::{Rawtext, Rcdata, ScriptData};
use crate::{Document, Fragment, LocalNodeId, Namespace as HtmlNamespace, Tree};

use destack_source::File;

use super::builder::{Handle, HtmlBuilder};
use super::insert::build_element;
use super::token::{SplitStatus, Token};

use std::borrow::Cow;
use std::cell::{Cell, RefCell};

/// One parser child insertion payload.
pub(crate) enum Child {
    /// One child node handle.
    Node(Handle),
    /// One child text payload.
    Text(HtmlString),
}

/// One HTML insertion mode.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub(crate) enum InsertionMode {
    /// The initial insertion mode.
    Initial,
    /// The before-html insertion mode.
    BeforeHtml,
    /// The before-head insertion mode.
    BeforeHead,
    /// The in-head insertion mode.
    InHead,
    /// The in-head-noscript insertion mode.
    InHeadNoscript,
    /// The after-head insertion mode.
    AfterHead,
    /// The in-body insertion mode.
    InBody,
    /// The text insertion mode.
    Text,
    /// The in-table insertion mode.
    InTable,
    /// The in-table-text insertion mode.
    InTableText,
    /// The in-caption insertion mode.
    InCaption,
    /// The in-column-group insertion mode.
    InColumnGroup,
    /// The in-table-body insertion mode.
    InTableBody,
    /// The in-row insertion mode.
    InRow,
    /// The in-cell insertion mode.
    InCell,
    /// The in-template insertion mode.
    InTemplate,
    /// The after-body insertion mode.
    AfterBody,
    /// The in-frameset insertion mode.
    InFrameset,
    /// The after-frameset insertion mode.
    AfterFrameset,
    /// The after-after-body insertion mode.
    AfterAfterBody,
    /// The after-after-frameset insertion mode.
    AfterAfterFrameset,
}

/// One parser step result.
pub(crate) enum ProcessResult<Handle> {
    /// The token was fully consumed.
    Done,
    /// The token was consumed and acknowledged one self-closing flag.
    DoneAckSelfClosing,
    /// The token should be split into whitespace and non-whitespace runs.
    SplitWhitespace(HtmlString),
    /// The token should be reprocessed in one new insertion mode.
    Reprocess(InsertionMode, Token),
    /// The parser should yield one script node.
    Script(Handle),
    /// The parser should switch the lexer to plaintext mode.
    ToPlaintext,
    /// The parser should switch the lexer to one raw-data mode.
    ToRawData(RawKind),
    /// The parser found one encoding indicator.
    EncodingIndicator(HtmlString),
}

/// One parser entry in the inline-element list.
#[derive(Debug)]
pub(crate) enum InlineEntry<Handle> {
    /// One active inline element.
    Element(Handle, Tag),
    /// One inline marker entry.
    Marker,
}

/// One document quirks mode.
#[derive(PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum QuirksMode {
    /// Full quirks mode.
    Quirks,
    /// Almost standards mode.
    LimitedQuirks,
    /// Standards mode.
    NoQuirks,
}

pub(crate) use self::QuirksMode::{LimitedQuirks, NoQuirks, Quirks};

/// Special properties of one element.
#[derive(Default)]
#[non_exhaustive]
pub(crate) struct ElementFlags {
    /// Whether this element is one template.
    pub template: bool,
    /// Whether this element is one MathML annotation XML integration point.
    pub mathml_annotation_xml_integration_point: bool,
    /// Whether duplicate attributes were encountered during tokenization.
    pub had_duplicate_attributes: bool,
}

/// HTML parser options.
#[derive(Debug, Copy, Clone)]
pub struct ParserOptions {
    /// Report all parse errors described in the spec at some performance cost.
    pub exact_errors: bool,

    /// Whether scripting is enabled.
    ///
    /// This changes how `<noscript>` elements are parsed.
    /// When scripting is enabled, the contents of `<noscript>` are parsed as text.
    /// When scripting is disabled, the contents of `<noscript>` are parsed as normal child nodes.
    pub scripting_enabled: bool,

    /// Whether this document is being parsed from an `<iframe srcdoc>` attribute.
    ///
    /// This affects heuristics that infer `QuirksMode` from `<!DOCTYPE>`.
    pub iframe_srcdoc: bool,

    /// Whether the parsed document type should be omitted from the tree.
    pub drop_doctype: bool,

    /// The initial parser quirks mode.
    pub quirks_mode: QuirksMode,
}

impl Default for ParserOptions {
    fn default() -> ParserOptions {
        ParserOptions {
            exact_errors: false,
            scripting_enabled: true,
            iframe_srcdoc: false,
            drop_doctype: false,
            quirks_mode: NoQuirks,
        }
    }
}

/// One HTML parser engine.
#[derive(Debug)]
pub struct Parser<'a> {
    /// Options controlling the behavior of the parser engine.
    pub(super) options: ParserOptions,

    /// Builder for tree modifications.
    pub(super) builder: HtmlBuilder<'a>,

    /// The authored HTML source.
    source: &'a str,

    /// The tokenizer options for this parse.
    lexer_options: LexerOptions,

    /// The current insertion mode.
    pub(super) mode: Cell<InsertionMode>,

    /// The original insertion mode used by text-like modes.
    pub(super) orig_mode: Cell<Option<InsertionMode>>,

    /// The stack of template insertion modes.
    pub(super) template_modes: RefCell<Vec<InsertionMode>>,

    /// The pending table character tokens.
    pub(super) pending_table_text: RefCell<Vec<(SplitStatus, HtmlString)>>,

    /// The quirks mode as set by the parser.
    pub(super) quirks_mode: Cell<QuirksMode>,

    /// The document node created by the builder.
    pub(super) document_handle: Handle,

    /// The stack of open elements, with the most recent at the end.
    pub(super) open_elements: RefCell<Vec<Handle>>,

    /// The list of active inline elements.
    pub(super) inline_elements: RefCell<Vec<InlineEntry<Handle>>>,

    /// The head element pointer.
    pub(super) head_element: RefCell<Option<Handle>>,

    /// The form element pointer.
    pub(super) form_element: RefCell<Option<Handle>>,

    /// Whether framesets are still allowed.
    pub(super) frameset_ok: Cell<bool>,

    /// Whether to ignore one following line feed.
    pub(super) ignore_lf: Cell<bool>,

    /// Whether foster parenting is enabled.
    pub(super) foster_parenting: Cell<bool>,

    /// The context element for the fragment parsing algorithm.
    pub(super) context_element: RefCell<Option<Handle>>,

    /// The current source line reported by the lexer.
    pub(super) current_line: Cell<u64>,
}

#[allow(clippy::clone_on_copy)]
impl<'a> Parser<'a> {
    /// Create one HTML parser over one direct HTML builder.
    pub(crate) fn new_with_options(
        builder: HtmlBuilder<'a>,
        source: &'a str,
        options: ParserOptions,
    ) -> Parser<'a> {
        let document_handle = builder.document_handle();
        Parser {
            options,
            builder,
            source,
            lexer_options: LexerOptions::default(),
            mode: Cell::new(InsertionMode::Initial),
            orig_mode: Cell::new(None),
            template_modes: Default::default(),
            pending_table_text: Default::default(),
            quirks_mode: Cell::new(options.quirks_mode),
            document_handle,
            open_elements: Default::default(),
            inline_elements: Default::default(),
            head_element: Default::default(),
            form_element: Default::default(),
            frameset_ok: Cell::new(true),
            ignore_lf: Default::default(),
            foster_parenting: Default::default(),
            context_element: Default::default(),
            current_line: Cell::new(1),
        }
    }

    /// Create one HTML parser from authored source.
    pub fn new(file: &'a File, source: &'a str) -> Self {
        let builder = HtmlBuilder::new(file, source);

        Self::new_with_options(builder, source, ParserOptions::default())
    }

    /// Create one HTML fragment parser from authored source and one context element.
    pub(crate) fn new_fragment_with_options(
        file: &'a File,
        source: &'a str,
        context: &ParseContextName,
        options: ParserOptions,
    ) -> Self {
        let builder = HtmlBuilder::new(file, source);
        let mut parser = Self::new_with_options(builder, source, options);
        let context = lower_context_name(context);
        let context_is_template = context.expanded() == expanded_name!(html "template");
        let context_is_form = context.expanded() == expanded_name!(html "form");
        let context_handle = build_element(&parser.builder, context, Vec::new());

        // fragment context
        if context_is_template {
            parser
                .template_modes
                .borrow_mut()
                .push(InsertionMode::InTemplate);
        }

        if context_is_form {
            *parser.form_element.borrow_mut() = Some(context_handle);
        }

        *parser.context_element.borrow_mut() = Some(context_handle);
        parser.lexer_options.initial_state = Some(parser.lexer_mode_for_context());
        parser.create_root(Vec::new());
        parser.mode.set(parser.reset_insertion_mode());

        parser
    }

    /// Create one HTML fragment parser from authored source and one context element.
    pub fn new_fragment(file: &'a File, source: &'a str, context: &ParseContextName) -> Self {
        Self::new_fragment_with_options(file, source, context, ParserOptions::default())
    }

    /// Run the lexer to completion for this parser instance.
    fn run(self) -> Lexer<Self> {
        let source = self.source;
        let lexer_options = self.lexer_options.clone();
        let lexer = Lexer::new(self, lexer_options);
        let input_buffer = BufferQueue::default();

        // one-shot input
        input_buffer.push_back(HtmlString::from_slice(source));

        // tokenization loop
        loop {
            if matches!(lexer.feed(&input_buffer), LexerResult::Done) {
                break;
            }
        }

        // trailing input
        if !input_buffer.is_empty() {
            lexer
                .parser
                .builder
                .parse_error(Cow::from("Parser finished with remaining input"));
        }

        // parser finish
        lexer.end();
        lexer
    }

    /// Parse one HTML document tree from authored source.
    pub fn parse(self) -> (Tree, LocalNodeId<Document>) {
        let lexer = self.run();
        lexer.parser.finish()
    }

    /// Parse one HTML fragment tree from authored source.
    pub fn parse_fragment(self) -> (Tree, LocalNodeId<Fragment>) {
        let (mut tree, document) = self.parse();
        let children = tree.get(document).children.clone();
        let span = tree.span(document);
        let fragment = tree.insert(Fragment { children }, span);

        (tree, fragment)
    }

    /// Finish parsing and return the direct HTML tree.
    pub(crate) fn finish(self) -> (Tree, LocalNodeId<Document>) {
        self.builder.finish()
    }

    /// Parse one HTML document and return one html5lib-style tree-construction snapshot.
    #[cfg(test)]
    pub(crate) fn parse_tree_construction(self) -> String {
        let lexer = self.run();

        lexer.parser.builder.finish_tree_construction()
    }

    /// Parse one HTML fragment and return one html5lib-style fragment snapshot.
    #[cfg(test)]
    pub(crate) fn parse_fragment_tree_construction(self) -> String {
        let lexer = self.run();

        lexer.parser.builder.finish_tree_construction_fragment()
    }

    /// Return the tokenizer mode for the current fragment context.
    fn lexer_mode_for_context(&self) -> LexerMode {
        let context = self.context_element.borrow();
        let Some(context) = context.as_ref() else {
            return Data;
        };
        let Some(context_name) = self.open_element_name(context) else {
            return Data;
        };

        // raw-text context
        match context_name.expanded() {
            expanded_name!(html "title") | expanded_name!(html "textarea") => RawData(Rcdata),
            expanded_name!(html "style")
            | expanded_name!(html "xmp")
            | expanded_name!(html "iframe")
            | expanded_name!(html "noembed")
            | expanded_name!(html "noframes") => RawData(Rawtext),
            expanded_name!(html "script") => RawData(ScriptData),
            expanded_name!(html "noscript") if self.options.scripting_enabled => RawData(Rawtext),
            expanded_name!(html "plaintext") => Plaintext,
            _ => Data,
        }
    }

    /// Return whether this parser is parsing one HTML fragment.
    pub(crate) fn is_fragment(&self) -> bool {
        self.context_element.borrow().is_some()
    }
}

/// One fragment context name with plain owned strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseContextName {
    /// The optional qualified prefix.
    pub prefix: Option<String>,
    /// The resolved namespace.
    pub namespace: HtmlNamespace,
    /// The local name.
    pub local: String,
}

/// Lower one authored HTML name into one parser qualified name.
fn lower_context_name(name: &ParseContextName) -> QualifiedName {
    // namespace prefix
    let prefix = name.prefix.as_deref().map(namespace_prefix_from_string);

    // resolved namespace
    let namespace = match &name.namespace {
        HtmlNamespace::Html => ns!(html),
        HtmlNamespace::Svg => ns!(svg),
        HtmlNamespace::MathMl => ns!(mathml),
        HtmlNamespace::Xml => ns!(xml),
        HtmlNamespace::XmlNs => ns!(xmlns),
        HtmlNamespace::XLink => ns!(xlink),
        HtmlNamespace::Other(value) => Namespace::from(value.as_str()),
    };

    QualifiedName::new(prefix, namespace, LocalName::from(name.local.as_str()))
}

/// Lower one authored namespace prefix into one parser prefix.
fn namespace_prefix_from_string(prefix: &str) -> Prefix {
    // well-known prefixes
    match prefix {
        "xml" => namespace_prefix!(xml),
        "xmlns" => namespace_prefix!(xmlns),
        "xlink" => namespace_prefix!(xlink),
        value => Prefix::from(value),
    }
}

/// Parse one HTML document tree from authored source.
pub fn parse_html(file: &File, source: &str) -> (Tree, LocalNodeId<Document>) {
    Parser::new(file, source).parse()
}

/// Parse one HTML document tree from authored source with explicit options.
pub fn parse_html_with_options(
    file: &File,
    source: &str,
    options: ParserOptions,
) -> (Tree, LocalNodeId<Document>) {
    let builder = HtmlBuilder::new(file, source);

    Parser::new_with_options(builder, source, options).parse()
}

/// Parse one HTML fragment tree from authored source and one context element.
pub fn parse_fragment(
    file: &File,
    source: &str,
    context: &ParseContextName,
) -> (Tree, LocalNodeId<Fragment>) {
    Parser::new_fragment(file, source, context).parse_fragment()
}

/// Parse one HTML fragment tree from authored source with explicit options.
pub fn parse_fragment_with_options(
    file: &File,
    source: &str,
    context: &ParseContextName,
    options: ParserOptions,
) -> (Tree, LocalNodeId<Fragment>) {
    Parser::new_fragment_with_options(file, source, context, options).parse_fragment()
}
