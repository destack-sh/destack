use crate::parse::context::FunctionContext;
use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserResult};

use destack_dir::{
    BlockContext, BlockForm, Declaration, LocalNodeId, ModuleDeclaration, NodeType, TokenType,
};

impl Parser {
    /// Parse a module declaration after its `module` head.
    ///
    /// Examples:
    /// ```ds
    /// module Network {}
    /// ```
    pub(crate) fn parse_module(
        &mut self,
        start: &ParseStart,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .parse_block_body(BlockForm::Explicit, BlockContext::Statement, function)
            .in_node(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let declaration = Declaration::Module(ModuleDeclaration { expressions });
        let declaration_id = self.insert_node(declaration, self.range_since(start));

        Ok(declaration_id)
    }
}
