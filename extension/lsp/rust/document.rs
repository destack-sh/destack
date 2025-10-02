use dyst_ast::{Module, ModuleFormat, NodeId, NodeTree, Parser};
use dyst_session::Session;
use dyst_source::{MultiSpan, Source};
use dyst_token::{TokenSpan, TokenType};

#[derive(Debug, Clone)]
pub struct Document {
    /// The format of the document.
    pub format: SourceFormat,
    /// The source of the document.
    pub source: Source,
    /// Whether the document content is controlled by an open editor session.
    pub is_open: bool,
    /// The semantic tokens of the document.
    pub tokens: Vec<TokenSpan>,
    /// The side tokens of the document.
    pub side_tokens: Vec<TokenSpan>,
    /// The side span of the document.
    pub side_span: MultiSpan,
    /// All tokens of the document.
    pub all_tokens: Vec<TokenSpan>,
    /// The AST of the document.
    pub ast: NodeTree,
    /// The root module ID of the document.
    pub module_id: Option<NodeId<Module>>,
}

impl Document {
    /// Parse the document from source.
    pub fn parse(source: Source, session: &mut Session, is_open: bool) -> Self {
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
        Document {
            format: source.format,
            source,
            is_open,
            tokens,
            side_tokens,
            side_span,
            all_tokens,
            ast,
            module_id,
        }
    }
}
