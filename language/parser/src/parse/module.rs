use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    BlockContext, BlockForm, Declaration, LocalNodeId, ModuleDeclaration, NodeType, TokenType,
};

impl Parser {
    /// Eat a module declaration.
    pub(crate) fn eat_module(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        self.eat_identifier_str("module")?;

        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .eat_block_body_in_context(BlockForm::Explicit, BlockContext::Statement)
            .for_node_type(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let declaration = Declaration::Module(ModuleDeclaration { expressions });
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));

        Ok(declaration_id)
    }
}
