use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    BlockContext, BlockForm, Declaration, LocalNodeId, ModuleDeclaration, NodeType, TokenType,
};

impl Parser {
    /// Eat a module directive declaration.
    pub(crate) fn eat_module_directive(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        self.eat_identifier_str("module")?;

        self.eat_token(TokenType::OpenBrace)?;
        let flags = self.flags.with_module_directive(true);
        let expressions = self
            .with_flags(flags, |parser| {
                parser.eat_block_body_in_context(BlockForm::Explicit, BlockContext::Statement)
            })
            .for_node_type(NodeType::Block)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let declaration = Declaration::Module(ModuleDeclaration { expressions });
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));

        Ok(declaration_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Declaration, Expression, ModuleDeclaration};
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node};

    #[test]
    fn test_parse_module_newline_as_identifiers() {
        let mut test = TestParser::new_with_language("module\nFoo\n{}", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let expressions = parser.parse();
        assert_eq!(expressions.len(), 3);

        let module_id = expressions[0];
        assert_expression_path!(parser, parser.tree.get(module_id), "module");

        let name_id = expressions[1];
        assert_expression_path!(parser, parser.tree.get(name_id), "Foo");

        let object_id = expressions[2];
        assert_node!(parser.tree, object_id, Expression::Block(_) => {});
    }

    #[test]
    fn test_parse_module_declaration() {
        let mut test = TestParser::new(
            r###"
module {
    tree: HtmlTree
}
"###,
        );
        let mut parser = test.prepare();

        let expressions = parser.parse();
        assert_eq!(expressions.len(), 1);

        let expression_id = expressions[0];
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Module(ModuleDeclaration { expressions }) => {
                assert_eq!(expressions.len(), 1);
            });
        });
    }
}
