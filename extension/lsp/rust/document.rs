use dyst_language_ast::{Module, ModuleFormat, NodeId, NodeTree, Parser};
use dyst_language_session::Session;
use dyst_language_source::Source;
use dyst_language_token::{TokenSpan, TokenType};

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
        let module_name = session.intern_string(module_name);

        // parse the document AST
        let mut parser = Parser::from_source(&source, session);
        let module_id = parser.with_recovery(
            parser.mark(),
            |parser| {
                parser
                    .eat_module_body(None, Some(module_name), ModuleFormat::Implicit)
                    .map(Some)
            },
            None,
            TokenType::End,
        );

        // turn into document
        let tokens = parser.tokens;
        let mut combined_tokens = Vec::with_capacity(tokens.len() + parser.trivia_tokens.len());
        combined_tokens.extend(tokens.iter());
        combined_tokens.extend(parser.trivia_tokens);
        combined_tokens.sort_by_key(|token| token.span.start);
        let ast = parser.tree;
        Document {
            source,
            is_open,
            tokens,
            combined_tokens,
            ast,
            module_id,
        }
    }
}
