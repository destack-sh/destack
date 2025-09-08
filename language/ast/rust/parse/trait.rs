use dyst_language_token::TokenType;

use crate::parse::ParserOptions;
use crate::{BlockFormat, Keyword, NodeId, ParseResult, Parser, Trait, Type, With};

impl<'a> Parser<'a> {
    /// Eat a Trait.
    ///
    /// Examples:
    /// ```
    /// trait { // anonymous trait
    ///     ...
    /// }
    ///
    /// trait Foo: Bar, Boz { // Foo is a subtype of Bar and Boz
    ///     let x: int32 // constant
    ///     function foo() => int32
    ///
    ///     function myFunc() { // nested declaration, default implementation
    ///     }
    /// }
    ///
    /// trait Baz<T> with T: Copy {
    ///     function baz() => T // semicolon optional
    /// }
    /// ```
    pub fn eat_trait(&mut self) -> ParseResult<NodeId<Trait>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Trait)?;

        // optional name
        let name = if self.peek_identifier().is_ok() {
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // optional static parameters: < ... >
        let static_parameters = if self.peek_token(TokenType::LessThan).is_ok() {
            self.bump(); // eat less than
            let params = self.with_options(
                ParserOptions {
                    in_static_type: true,
                    ..self.options
                },
                |p| p.eat_parameters_body(),
            )?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(params)
        } else {
            None
        };

        // optional supertraits after ':'
        let mut supertraits: Vec<NodeId<Type>> = Vec::new();
        if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            // first supertrait
            let ty = self.eat_type()?;
            supertraits.push(ty);
            // more, comma-separated
            while self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                let ty = self.eat_type()?;
                supertraits.push(ty);
            }
        }

        // optional with declarations in header
        let mut withs: Vec<NodeId<With>> = Vec::new();
        if self.peek_keyword(Keyword::With).is_ok() {
            self.bump(); // eat with
            let with = self.eat_with_body()?;
            withs.push(with);
        }

        // body
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let statements = self.eat_block_body(BlockFormat::Explicit)?;
        self.eat_token(TokenType::CloseBrace)?;

        let trait_id = self.tree.allocate(
            Trait {
                name,
                static_parameters,
                supertraits,
                withs,
                statements,
            },
            self.get_span_from(start),
        );
        Ok(trait_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Function, Statement, Trait, Type, WithClause, assert_node};

    #[test]
    fn test_parse_trait_anonymous_empty() {
        let test = TestParser::new("trait {}");
        let mut parser = test.parser();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, supertraits, withs, statements } => {
            assert!(name.is_none());
            assert!(static_parameters.is_none());
            assert!(supertraits.is_empty());
            assert!(withs.is_empty());
            assert!(statements.is_empty());
        });
    }

    #[test]
    fn test_parse_trait_with_name_and_supertraits() {
        let test = TestParser::new("trait Foo: Bar, Boz {}");
        let mut parser = test.parser();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, supertraits, .. } => {
            assert_eq!(*name, Some(parser.strings.intern("Foo")));
            assert_eq!(supertraits.len(), 2);

            // Bar
            assert_node!(parser.tree, supertraits[0], Type::Path { path, .. } => {
                assert_eq!(
                    *path,
                    parser.paths.intern(vec![parser.strings.intern("Bar")])
                );
            });

            // Boz
            assert_node!(parser.tree, supertraits[1], Type::Path { path, .. } => {
                assert_eq!(
                    *path,
                    parser.paths.intern(vec![parser.strings.intern("Boz")])
                );
            });
        });
    }

    #[test]
    fn test_parse_trait_with_members() {
        let test = TestParser::new(
            r###"
trait Foo {
    let x: int32 = 4

    use Baz

    function foo() => int32
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, statements, .. } => {
            assert_eq!(*name, Some(parser.strings.intern("Foo")));
            assert_eq!(statements.len(), 3);
        });
    }

    #[test]
    fn test_parse_trait_with_static_parameters() {
        let test = TestParser::new("trait Baz<T> {}");
        let mut parser = test.parser();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, .. } => {
            assert_eq!(*name, Some(parser.strings.intern("Baz")));
            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);
        });
    }

    #[test]
    fn test_parse_trait_with_clause() {
        let test = TestParser::new(
            r###"
trait Baz<T> with T: Copy {
    function baz() => T
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, withs, statements, .. } => {
            assert_eq!(*name, Some(parser.strings.intern("Baz")));

            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);

            assert_eq!(withs.len(), 1);

            // with T: Copy
            let with_id = withs[0];
            let with = parser.tree.get(with_id);
            assert_eq!(with.clauses.len(), 1);

            assert_node!(parser.tree, with.clauses[0], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Type::Path { path, .. } => {
                    assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
                });
                assert_node!(parser.tree, *assertion, Type::Path { path, .. } => {
                    assert_eq!(
                        *path,
                        parser.paths.intern(vec![parser.strings.intern("Copy")])
                    );
                });
            });

            assert_eq!(statements.len(), 1);

            // function baz() => T
            let statement_id = statements[0];
            assert_node!(parser.tree, statement_id, Statement::Function(func_id) => {
                assert_node!(parser.tree, *func_id, Function { return_type, .. } => {
                    let ret = return_type.expect("expected return type");
                    assert_node!(parser.tree, ret, Type::Path { path, .. } => {
                        assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
                    });
                })
            })
        });
    }
}
