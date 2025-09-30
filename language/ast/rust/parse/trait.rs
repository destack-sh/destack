use dyst_token::TokenType;

use crate::parse::prelude::*;
use crate::{
    BlockFormat, Expression, Keyword, NodeId, NodeType, ParseResult, Parser, Trait, Visibility,
    With,
};

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
    pub fn eat_trait(&mut self, visibility: Option<Visibility>) -> ParseResult<NodeId<Trait>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Trait)
            .for_node_type(NodeType::Trait)?;

        // optional name
        let name = self.eat_identifier_or_wildcard_maybe()?;

        // optional static parameters: < ... >
        let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
            self.bump(); // eat less than
            let params = self
                .with_options(self.options.in_static_type(), |parser| {
                    parser.eat_parameters_body()
                })
                .for_node_type(NodeType::Trait)?;
            self.eat_token(TokenType::GreaterThan)
                .for_node_type(NodeType::Trait)?;
            Some(params)
        } else {
            None
        };

        // optional super types: : ...
        let super_types = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let mut super_types: Vec<NodeId<Expression>> = Vec::new();
            loop {
                // eat until open parenthesis
                if self.peek_token(TokenType::OpenBrace).is_ok() {
                    break;
                }
                // consume any stop
                else if self.peek_any_stop().is_ok() {
                    self.eat_any_stop_with_newlines()?;
                }
                // keep eating super types
                else {
                    let super_type = self
                        .with_options(self.options.in_before_block(), |parser| {
                            parser.eat_expression()
                        })
                        .for_node_type(NodeType::Trait)?;
                    super_types.push(super_type);
                }
            }
            Some(super_types)
        } else {
            None
        };

        // optional with declarations in header
        let mut withs: Vec<NodeId<With>> = Vec::new();
        if self.peek_keyword(Keyword::With).is_ok() {
            self.bump(); // eat with
            let with = self.eat_with_body().for_node_type(NodeType::Trait)?;
            withs.push(with);
        }

        // body
        self.eat_token(TokenType::OpenBrace)
            .for_node_type(NodeType::Trait)?;
        self.eat_newlines_maybe()?;
        let statements = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Trait)?;

        let trait_id = self.tree.allocate(
            Trait {
                name,
                visibility,
                static_parameters,
                super_types,
                withs,
                expressions: statements,
            },
            self.get_span_from(start),
        );
        Ok(trait_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Expression, Function, Trait, WithClause, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_trait_anonymous_empty() {
        let mut test = TestParser::new("trait {}");
        let mut parser = test.prepare();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, withs, expressions, .. } => {
            assert!(name.is_none());
            assert!(static_parameters.is_none());
            assert!(withs.is_empty());
            assert!(expressions.is_empty());
        });
    }

    #[test]
    fn test_parse_trait_with_super_types() {
        let mut test = TestParser::new("trait Foo: Bar {}");
        let mut parser = test.prepare();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, super_types, expressions, .. } => {
            assert_eq!(expressions.len(), 0);
            assert_string!(parser.session, name.unwrap(), "Foo");
            assert!(expressions.is_empty());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Bar");
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
        assert_node!(parser.tree, trait_id, Trait { name, expressions, super_types, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");
            assert_eq!(expressions.len(), 3);

            // : Baz
            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Baz");
            });
        });
    }

    #[test]
    fn test_parse_trait_with_static_parameters() {
        let mut test = TestParser::new("trait Baz<T> {}");
        let mut parser = test.prepare();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, .. } => {
            assert_string!(parser.session, name.unwrap(), "Baz");
            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);
        });
    }

    #[test]
    fn test_parse_trait_with_clause() {
        let mut test = TestParser::new(
            r###"
trait Baz<T> with T: Copy {
    function baz() => T
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait(None).unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, withs, expressions, .. } => {
            assert_string!(parser.session, name.unwrap(), "Baz");

            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);

            assert_eq!(withs.len(), 1);

            // with T: Copy
            let with_id = withs[0];
            let with = parser.tree.get(with_id);
            assert_eq!(with.clauses.len(), 1);
            // with T: Copy
            assert_node!(parser.tree, with.clauses[0], WithClause::Assertion { target, assertion } => {
                // T
                assert_node!(parser.tree, *target, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "T");
                });
                // Copy
                assert_node!(parser.tree, *assertion, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Copy");
                });
            });

            assert_eq!(expressions.len(), 1);

            // function baz() => T
            let expression_id = expressions[0];
            assert_node!(parser.tree, expression_id, Expression::Function(func_id) => {
                assert_node!(parser.tree, *func_id, Function { name, return_type, .. } => {
                    // baz
                    assert_string!(parser.session, name.unwrap(), "baz");
                    // => T
                    let ret = return_type.expect("expected return type");
                    assert_node!(parser.tree, ret, Expression::Path { path, .. } => {
                        assert_path!(parser.session, *path, "T");
                    });
                });
            })
        });
    }
}
