use crate::parse::prelude::*;
use dyst_language_source::StringId;
use dyst_language_token::TokenType;

use crate::{
    BlockFormat, Keyword, Module, ModuleFormat, NodeId, NodeType, ParseResult, Parser, Visibility,
};

impl<'a> Parser<'a> {
    /// Eat a module declaration (incl. `module` keyword).
    pub fn eat_module(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Module>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Module)
            .for_node_type(NodeType::Module)?;
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };
        let module = {
            if self.peek_token(TokenType::OpenBrace).is_ok() {
                self.bump(); // eat open brace
                let statements = self
                    .eat_block_body(BlockFormat::Explicit)
                    .for_node_type(NodeType::Block)?;
                self.eat_token(TokenType::CloseBrace)
                    .for_node_type(NodeType::Module)?;
                Module {
                    format: ModuleFormat::Inline,
                    name,
                    visibility,
                    statements,
                }
            } else {
                Module {
                    format: ModuleFormat::Forward,
                    name,
                    visibility,
                    statements: Vec::new(),
                }
            }
        };
        let module_id = self.tree.allocate(module, self.get_span_from(start));
        Ok(module_id)
    }

    // Eat a module body (aka a module file, without `module` keyword or braces).
    pub fn eat_module_body(
        &mut self,
        visibility: Option<Visibility>,
        name: Option<StringId>,
        format: ModuleFormat,
    ) -> ParseResult<NodeId<Module>> {
        let start = self.mark();
        let statements = self
            .eat_block_body(BlockFormat::Implicit)
            .for_node_type(NodeType::Block)?;
        let module = Module {
            format,
            name,
            visibility,
            statements,
        };
        let module_id = self.tree.allocate(module, self.get_span_from(start));
        Ok(module_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Module, ModuleFormat, assert_node, assert_string};

    #[test]
    fn test_parse_empty_module() {
        let mut test = TestParser::new("module { }");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(None).unwrap();
        assert_node!(parser.tree, module_id, Module { name, visibility, statements, format, .. } => {
            assert!(name.is_none());
            assert!(visibility.is_none());
            assert!(statements.is_empty());
            assert_eq!(*format, ModuleFormat::Inline);
        });
    }

    #[test]
    fn test_parse_forward_module() {
        let mut test = TestParser::new("module x;");
        let mut parser = test.prepare();
        let module_id = parser.eat_module(None).unwrap();
        assert_node!(parser.tree, module_id, Module { name, visibility, statements, format, .. } => {
            assert_string!(parser.session, name.unwrap(), "x");
            assert!(visibility.is_none());
            assert!(statements.is_empty());
            assert_eq!(*format, ModuleFormat::Forward);
        });
    }
}
