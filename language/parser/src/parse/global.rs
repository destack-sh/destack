use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserResult};

use tspp_dir::{
    BlockContext, BlockForm, Declaration, GlobalDeclaration, LocalNodeId, NodeType, TokenType,
};
use tspp_source::ByteRange;

use super::DeclarationHeader;

impl Parser {
    /// Parse a global declaration after its `global` head.
    ///
    /// Examples:
    /// ```tspp
    /// global { interface Window {} }
    /// ```
    pub(crate) fn parse_global(
        &mut self,
        start: &ParseStart,
        main_range: ByteRange,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .parse_block_body(BlockForm::Explicit, BlockContext::Statement)
            .in_node(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let global = Declaration::Global(GlobalDeclaration {
            is_ambient: header.is_ambient,
            expressions,
        });
        let global_id = self.insert_node(global, self.range_since(start));
        self.tree.set_main_range(global_id, main_range);

        Ok(global_id)
    }
}
