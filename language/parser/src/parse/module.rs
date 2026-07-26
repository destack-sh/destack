use crate::parse::context::FunctionContext;
use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserResult};

use destack_dir::{
    BlockContext, BlockForm, Declaration, LocalNodeId, ModuleDeclaration, NodeType, TokenType,
};
use destack_source::ByteRange;

impl Parser {
    /// Parse a module declaration after its `module` head.
    ///
    /// Examples:
    /// ```ds
    /// module {}
    /// ```
    pub(crate) fn parse_module(
        &mut self,
        start: &ParseStart,
        main_range: ByteRange,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .parse_block_body(BlockForm::Explicit, BlockContext::Statement, function)
            .in_node(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let declaration = Declaration::Module(ModuleDeclaration { expressions });
        let declaration_id = self.insert_node(declaration, self.range_since(start));
        self.tree.set_main_range(declaration_id, main_range);

        Ok(declaration_id)
    }
}
