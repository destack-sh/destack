use crate::parse::prelude::*;
use crate::{Parser, ParserResult, ParserSpanStart};

use destack_dir::{
    BlockContext, BlockForm, Declaration, GlobalDeclaration, LocalNodeId, NodeType, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::DeclarationHeader;

impl Parser {
    /// Eat a global augmentation declaration.
    pub(crate) fn eat_global(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        self.eat_identifier_str("global")?;

        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .eat_block_body_in_context(BlockForm::Explicit, BlockContext::Statement)
            .for_node_type(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let global = Declaration::Global(GlobalDeclaration {
            is_ambient: header.is_ambient,
            expressions,
        });
        let global_id = self.insert_node(global, self.get_span_from(start));

        if let Some(span) = header.declare_span {
            self.tree.set_side_span(
                global_id,
                NodeSpanType::Region(NodeSpanRegion::Prelude),
                span,
            );
        }

        Ok(global_id)
    }
}
