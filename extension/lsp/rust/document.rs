use dyst_ast::{Module, ModuleFormat, NodeId, NodeTree, Parser};
use dyst_session::Session;
use dyst_source::Source;
use dyst_token::{TokenSpan, TokenType};

#[derive(Debug, Clone)]
pub struct Document {
    /// The source of the document.
    pub source: Source,
    /// Whether the document content is controlled by an open editor session.
    pub is_open: bool,
    /// The semantic tokens of the document.
    pub tokens: Vec<TokenSpan>,
    /// The combined tokens of the document.
    pub combined_tokens: Vec<TokenSpan>,
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
                    .eat_module_body(None, Some(module_name_id), ModuleFormat::Implicit)
                    .map(Some)
            },
            None,
            TokenType::End,
        );
        parser.finalize();

        // turn into document
        let tokens = parser.tokens;
        let mut all_tokens = Vec::with_capacity(tokens.len() + parser.side_tokens.len());
        all_tokens.extend(tokens.iter());
        all_tokens.extend(parser.side_tokens);
        all_tokens.sort_by_key(|token| token.span.start);
        let ast = parser.tree;
        Document {
            source,
            is_open,
            tokens,
            combined_tokens: all_tokens,
            ast,
            module_id,
        }
    }
}
