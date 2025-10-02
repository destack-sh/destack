use dyst_ast::{Module, ModuleFormat, NodeId, NodeTree, Parser};
use dyst_session::Session;
use dyst_source::{MultiSpan, Source, SourceFormat, SourceId, Uri};
use dyst_token::{TokenSpan, TokenType};

/// A document.
#[derive(Debug, Clone)]
pub struct Document {
    /// The ID of the source.
    pub id: SourceId,
    /// The URI of the SourceFile.
    pub uri: Uri,
    /// The format of the document.
    pub format: SourceFormat,
    /// Whether the document is currently open.
    pub is_open: bool,
    /// The content of the document.
    pub content: DocumentContent,
}

/// The content of a document.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DocumentContent {
    Text {
        /// The source of the document.
        source: Source,
        /// The tokens of the document.
        tokens: Vec<TokenSpan>,
        /// The side tokens of the document.
        side_tokens: Vec<TokenSpan>,
        /// The side span of the document.
        side_span: MultiSpan,
        /// All tokens of the document (including side tokens).
        all_tokens: Vec<TokenSpan>,
        /// The AST of the document.
        ast: NodeTree,
        /// The module ID of the document, when available.
        module_id: Option<NodeId<Module>>,
    },
    Binary {
        /// The content of the document.
        content: Vec<u8>,
    },
}

impl DocumentContent {
    /// Build a document for the provided text source.
    pub(crate) fn from_text(source: Source, session: &mut Session) -> Self {
        // module name
        let module_name = source.uri.last_segment().unwrap_or("<string>");
        let module_name_id = session.intern_string(module_name);

        // parse the document AST
        let mut parser = Parser::prepare(&source, session);
        let module_id = parser.with_recovery(
            parser.mark(),
            |parser| {
                parser
                    .eat_module_body(None, Some(module_name_id), ModuleFormat::Source)
                    .map(Some)
            },
            None,
            TokenType::End,
        );
        parser.finalize();

        // turn into document
        let side_span = parser.get_side_span();
        let side_tokens = parser.side_tokens;
        let tokens = parser.tokens;
        let mut all_tokens: Vec<TokenSpan> = Vec::with_capacity(tokens.len() + side_tokens.len());
        all_tokens.extend(tokens.iter());
        all_tokens.extend(side_tokens.iter());
        all_tokens.sort_by_key(|token| token.span.start);
        let ast = parser.tree;

        DocumentContent::Text {
            source,
            tokens,
            side_tokens,
            side_span,
            all_tokens,
            ast,
            module_id,
        }
    }

    /// Build a document for the provided binary source.
    pub(crate) fn from_binary(content: Vec<u8>) -> Self {
        DocumentContent::Binary { content }
    }
}

impl Document {
    /// Build a document for the provided text source.
    pub(crate) fn from_text(
        id: SourceId,
        uri: Uri,
        format: SourceFormat,
        is_open: bool,
        content: String,
        session: &mut Session,
    ) -> Self {
        let source = Source::from_string(id, uri.clone(), format, content);
        let content = DocumentContent::from_text(source, session);

        Document {
            id,
            uri,
            format,
            is_open,
            content,
        }
    }

    /// Build a document for the provided binary source.
    pub(crate) fn from_binary(
        id: SourceId,
        uri: Uri,
        format: SourceFormat,
        is_open: bool,
        content: Vec<u8>,
    ) -> Self {
        Document {
            id,
            uri,
            format,
            is_open,
            content: DocumentContent::from_binary(content),
        }
    }
}
