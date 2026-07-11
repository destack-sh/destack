use crate::parse::context::FunctionContext;
use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserResult};

use destack_dir::{
    BlockContext, BlockForm, Declaration, GlobalDeclaration, LocalNodeId, NodeType, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::DeclarationHeader;

impl Parser {
    /// Parse a global augmentation declaration after its `global` head.
    ///
    /// Examples:
    /// ```ds
    /// global { interface Window {} }
    /// ```
    pub(crate) fn parse_global(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .parse_block_body(BlockForm::Explicit, BlockContext::Statement, function)
            .in_node(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let global = Declaration::Global(GlobalDeclaration {
            is_ambient: header.is_ambient,
            expressions,
        });
        let global_id = self.insert_node(global, self.range_since(start));

        if let Some(range) = header.declare_range {
            self.tree.set_side_range(
                global_id,
                NodeSpanType::Region(NodeSpanRegion::Prelude),
                range,
            );
        }

        Ok(global_id)
    }
}
