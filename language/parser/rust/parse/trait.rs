use crate::TokenType;

use crate::parse::prelude::*;
use crate::{ParserResult, BlockFormat, Definition, Keyword, NodeId, NodeType, Parser, Visibility};

impl<'a> Parser<'a> {
    /// Eat a Trait.
    ///
    /// Examples:
    /// ```
    /// trait { // anonymous trait
    ///     ...
    /// }
    ///
    /// trait _ {} // explicit anonymous trait (for disambiguation)
    ///
    /// trait Foo {
    ///     use Bar, Boz // Foo *uses* Bar and Boz
    ///     
    ///     let x: int32 // constant
    ///     function foo() => int32
    ///
    ///     function myFunc() { // nested declaration, default implementation
    ///     }
    /// }
    ///
    /// trait Baz<T> {
    ///     use Bar
    ///
    ///     function baz() => T // semicolon optional
    /// }
    /// ```
    pub fn eat_trait(&mut self, visibility: Option<Visibility>) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Trait)
            .for_node_type(NodeType::Definition)?;

        // optional name
        let name = self.eat_identifier_or_wildcard_maybe()?;

        // optional static parameters: < ... >
        let static_parameters = self.eat_static_parameters_maybe()?;

        // optional super types: : ...
        let super_types = self.eat_super_types_maybe()?;

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.eat_token(TokenType::OpenBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let expressions = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        let trait_id = self.tree.allocate(
            Definition::Trait {
                name,
                visibility,
                static_parameters,
                super_types,
                with_clauses,
                where_clauses,
                expressions,
            },
            self.get_span_from(start),
        );
        Ok(trait_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Definition, Expression, WhereClause, WithClause, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_trait_anonymous_empty() {
        let mut test = TestParser::new("trait {}");
        let mut parser = test.prepare();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Definition::Trait { name, static_parameters, with_clauses, where_clauses, expressions, .. } => {
            assert!(name.is_none());
            assert!(static_parameters.is_none());
            assert!(with_clauses.is_none());
            assert!(where_clauses.is_none());
            assert!(expressions.is_empty());
        });
    }

    #[test]
    fn test_parse_trait_with_super_types() {
        let mut test = TestParser::new("trait Foo: Bar {}");
        let mut parser = test.prepare();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Definition::Trait { name, super_types, where_clauses, expressions, .. } => {
            assert_eq!(expressions.len(), 0);
            assert_string!(parser, name.unwrap(), "Foo");
            assert!(expressions.is_empty());
            assert!(where_clauses.is_none());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_trait_with_members() {
        let mut test = TestParser::new(
            r###"
trait Foo: Baz {
    let x: int32 = 4

    use Baz

    function foo() => int32
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Definition::Trait { name, expressions, super_types, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "Foo");
            assert_eq!(expressions.len(), 3);
            assert!(where_clauses.is_none());

            // : Baz
            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });
        });
    }

    #[test]
    fn test_parse_trait_with_static_parameters() {
        let mut test = TestParser::new("trait Baz<T> {}");
        let mut parser = test.prepare();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Definition::Trait { name, static_parameters, .. } => {
            assert_string!(parser, name.unwrap(), "Baz");
            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);
        });
    }

    #[test]
    fn test_parse_trait_with_clause() {
        let mut test = TestParser::new(
            r###"
trait Baz<T> with T: Copy where Requirement: Trait {
    function baz() => T
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Definition::Trait { name, static_parameters, with_clauses, where_clauses, expressions, .. } => {
            assert_string!(parser, name.unwrap(), "Baz");

            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);

            // with T: Copy
            let with_items = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);
            assert_node!(parser.tree, with_items[0], WithClause { alias, right } => {
                assert_string!(parser, alias.unwrap(), "T");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Copy");
                });
            });

            // where Requirement: Trait
            let where_items = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);
            assert_node!(parser.tree, where_items[0], WhereClause::Assertion { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Trait");
                });
            });

            assert_eq!(expressions.len(), 1);

            // function baz() => T
            let expression_id = expressions[0];
            assert_node!(parser.tree, expression_id, Expression::Definition(definition_id) => {
                assert_node!(parser.tree, *definition_id, Definition::Function { name, return_type, .. } => {
                    // baz
                    assert_string!(parser, name.unwrap(), "baz");
                    // => T
                    let ret = return_type.expect("expected return type");
                    assert_node!(parser.tree, ret, Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            })
        });
    }
}
