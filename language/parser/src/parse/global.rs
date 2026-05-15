use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    BlockContext, BlockForm, Declaration, GlobalDeclaration, LocalNodeId, NodeType, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use super::expression::common::DeclarationHeader;

impl Parser {
    /// Eat a global augmentation declaration.
    pub(crate) fn eat_global(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<Declaration>> {
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

#[cfg(test)]
mod tests {
    use destack_dir::{Declaration, Expression, GlobalDeclaration};
    use destack_source::LanguageType;

    use crate::{TestParser, assert_node};

    #[test]
    fn test_parse_declare_global_block() {
        let mut test = TestParser::new(
            r###"
declare global {
    interface Foo {
        bar(): boolean
    }
}
"###,
        );
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
                assert_eq!(*is_ambient, true);
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_global_block_without_declare() {
        let mut test = TestParser::new_with_language(
            r###"
global {
    interface Foo { }
}
"###,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.flags).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
                assert_eq!(*is_ambient, true);
                assert_eq!(expressions.len(), 1);
            });
        });
    }

    #[test]
    fn test_parse_global_declaration() {
        let mut test = TestParser::new(
            r###"
global {
    let process: Process
}
"###,
        );
        let mut parser = test.prepare();

        let expressions = parser.parse();
        assert_eq!(expressions.len(), 1);

        let expression_id = expressions[0];
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
                assert_eq!(*is_ambient, false);
                assert_eq!(expressions.len(), 1);
            });
        });
    }
}
