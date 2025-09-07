use dyst_language_token::TokenType;

use crate::{Function, Keyword, Let, NodeId, ParseResult, Parser, Trait, Type, Use, With};

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
    /// }
    ///
    /// trait Baz[T] with T: Copy {
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

        // optional static parameters: [ ... ]
        let static_parameters = if self.peek_token(TokenType::OpenBracket).is_ok() {
            self.eat_token(TokenType::OpenBracket)?;
            let params = self.eat_parameters_body()?;
            self.eat_token(TokenType::CloseBracket)?;
            Some(params)
        } else {
            None
        };

        // optional supertraits after ':'
        let mut supertraits: Vec<NodeId<Type>> = Vec::new();
        if self.peek_colon().is_ok() {
            self.eat_colon()?;
            // first supertrait
            let ty = self.eat_type()?;
            supertraits.push(ty);
            // more, comma-separated
            while self.peek_token(TokenType::Comma).is_ok() {
                self.eat_comma()?;
                let ty = self.eat_type()?;
                supertraits.push(ty);
            }
        }

        // optional with declarations in header
        let mut withs: Vec<NodeId<With>> = Vec::new();
        if self.peek_keyword(Keyword::With).is_ok() {
            self.eat_keyword(Keyword::With)?;
            let with = self.eat_with_body()?;
            withs.push(with);
        }

        // body
        let mut usings: Vec<NodeId<Use>> = Vec::new();
        let mut lets: Vec<NodeId<Let>> = Vec::new();
        let mut functions: Vec<NodeId<Function>> = Vec::new();
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            self.eat_token(TokenType::OpenBrace)?;
            self.eat_newlines_maybe()?;
            loop {
                // stop on closing brace
                if self.peek_token(TokenType::CloseBrace).is_ok() {
                    break;
                }
                // consume any stop
                else if self.peek_any_stop().is_ok() {
                    self.eat_any_stop_with_newlines()?;
                }
                // let
                else if self.peek_keyword(Keyword::Let).is_ok()
                    || self.peek_keyword(Keyword::Var).is_ok()
                {
                    let let_declaration = self.eat_let_or_var()?;
                    lets.push(let_declaration);
                }
                // use
                else if self.peek_keyword(Keyword::Use).is_ok() {
                    let using = self.eat_use()?;
                    usings.push(using);
                }
                // function
                else if self.peek_keyword(Keyword::Function).is_ok() {
                    let function = self.eat_function()?;
                    functions.push(function);
                } else {
                    // skip unexpected tokens conservatively
                    self.bump();
                }
            }
            self.eat_token(TokenType::CloseBrace)?;
        }

        let trait_id = self.tree.allocate(
            Trait {
                name,
                static_parameters,
                supertraits,
                withs,
                usings,
                lets,
                functions,
            },
            self.get_span_from(start),
        );
        Ok(trait_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Trait, Type, WithClause, assert_node};

    #[test]
    fn test_parse_trait_anonymous_empty() {
        let test = TestParser::new("trait {}");
        let mut parser = test.parser();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, supertraits, withs, usings, lets, functions } => {
            assert!(name.is_none());
            assert!(static_parameters.is_none());
            assert!(supertraits.is_empty());
            assert!(withs.is_empty());
            assert!(usings.is_empty());
            assert!(lets.is_empty());
            assert!(functions.is_empty());
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
        assert_node!(parser.tree, trait_id, Trait { name, usings, lets, functions, .. } => {
            assert_eq!(*name, Some(parser.strings.intern("Foo")));
            assert_eq!(usings.len(), 1);
            assert_eq!(lets.len(), 1);
            assert_eq!(functions.len(), 1);
        });
    }

    #[test]
    fn test_parse_trait_with_static_parameters() {
        let test = TestParser::new("trait Baz[T] {}");
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
trait Baz[T] with T: Copy {
    function baz() => T
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait().unwrap();
        assert_node!(parser.tree, trait_id, Trait { name, static_parameters, withs, functions, .. } => {
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

            assert_eq!(functions.len(), 1);

            // function baz() => T
            let func_id = functions[0];
            let func = parser.tree.get(func_id);
            let ret = func.return_type.expect("expected return type");
            assert_node!(parser.tree, ret, Type::Path { path, .. } => {
                assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
            });
        });
    }
}
