use dyst_ast::{
    Definition, DystFormatContext, DystFormatOptions, ModuleFormat, NodeId, NodeParentIndex,
    NodeTree, Parser, PathPool,
};
use dyst_fir::format;
use dyst_session::Session;
use dyst_source::{MultiSpan, Source, SourceFormat, SourceId, StringPool, Uri};
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
    pub body: DocumentBody,
}

/// The content of a document.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DocumentBody {
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
        /// The root definition ID of the document, when available.
        root_definition_id: Option<NodeId<Definition>>,
        /// The string pool.
        strings: StringPool,
        /// The path pool.
        paths: PathPool,
    },
    Binary {
        /// The content of the document.
        content: Vec<u8>,
    },
}

impl DocumentBody {
    /// Build a document for the provided text source.
    pub(crate) fn from_text(source: Source, session: &mut Session) -> Self {
        // module name
        let module_name = source.uri.last_segment().unwrap_or("<string>");

        // parse the document AST
        let mut parser = Parser::prepare(&source, session);
        let module_name_id = parser.intern_string(module_name);
        let root_definition_id = parser.with_recovery(
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

        // combine tokens and AST into document body
        let side_span = parser.get_side_span();
        let side_tokens = parser.side_tokens;
        let tokens = parser.tokens;
        let mut all_tokens: Vec<TokenSpan> = Vec::with_capacity(tokens.len() + side_tokens.len());
        all_tokens.extend(tokens.iter());
        all_tokens.extend(side_tokens.iter());
        all_tokens.sort_by_key(|token| token.span.start);
        let ast = parser.tree;
        let strings = parser.strings;
        let paths = parser.paths;

        DocumentBody::Text {
            source,
            tokens,
            side_tokens,
            side_span,
            all_tokens,
            ast,
            root_definition_id,
            strings,
            paths,
        }
    }

    /// Build a document for the provided binary source.
    pub(crate) fn from_binary(content: Vec<u8>) -> Self {
        DocumentBody::Binary { content }
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
        let content = DocumentBody::from_text(source, session);

        Document {
            id,
            uri,
            format,
            is_open,
            body: content,
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
            body: DocumentBody::from_binary(content),
        }
    }

    /// Format a (text) document.
    /// If the document couldn't be formatted, returns `None`.
    pub fn format(&self, session: &Session) -> Option<String> {
        let DocumentBody::Text {
            source,
            tokens,
            side_tokens,
            side_span,
            ast,
            root_definition_id: module_id,
            strings,
            paths,
            ..
        } = &self.body
        else {
            return None;
        };

        // format with default options
        // NOTE #Incomplete: configure LSP formatting options from Workspace
        let options = DystFormatOptions::default();
        let context = DystFormatContext {
            options,
            source,
            tokens,
            side_tokens,
            side_span,
            tree: ast,
            spans: &ast.spans,
            parents: NodeParentIndex::from_tree(ast),
            session,
            strings,
            paths,
        };
        let formatted = format!(context, [module_id]).unwrap();
        let printed = formatted.print();
        Some(printed.unwrap().as_str().to_string())
    }
}
