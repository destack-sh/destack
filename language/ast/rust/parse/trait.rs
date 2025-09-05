use destack_language_token::TokenType;

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
            let with = self.eat_with_header()?;
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
                    self.eat_any_stop()?;
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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Parser, Type};

    #[test]
    fn test_parse_trait_anonymous() {
        let input = r###"trait {
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);

        let trait_id = parser.eat_trait().unwrap();
        let tr = parser.tree.get(trait_id);
        assert!(tr.name.is_none());
        assert!(tr.static_parameters.is_none());
        assert!(tr.supertraits.is_empty());
        assert!(tr.withs.is_empty());
        assert!(tr.usings.is_empty());
        assert!(tr.lets.is_empty());
        assert!(tr.functions.is_empty());
    }

    #[test]
    fn test_parse_trait_with_supertraits_and_members() {
        let input = r###"
trait Foo: Bar, Boz {
    let x: int32 = 4

    use Baz

    function foo() => int32
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait().unwrap();
        let tr = parser.tree.get(trait_id);
        assert_eq!(tr.name, Some(parser.strings.intern("Foo")));
        assert_eq!(tr.supertraits.len(), 2);
        // Bar
        match parser.tree.get(tr.supertraits[0]) {
            Type::Path { path, .. } => {
                assert_eq!(
                    *path,
                    parser.paths.intern(vec![parser.strings.intern("Bar")])
                );
            }
            _ => panic!("expected path type"),
        }
        // Boz
        match parser.tree.get(tr.supertraits[1]) {
            Type::Path { path, .. } => {
                assert_eq!(
                    *path,
                    parser.paths.intern(vec![parser.strings.intern("Boz")])
                );
            }
            _ => panic!("expected path type"),
        }
        assert_eq!(tr.usings.len(), 1);
        assert_eq!(tr.lets.len(), 1);
        assert_eq!(tr.functions.len(), 1);
    }

    #[test]
    fn test_parse_trait_with_static_params_and_with() {
        let input = r###"
trait Baz[T] with T: Copy {
    function baz() => T
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let trait_id = parser.eat_trait().unwrap();
        let tr = parser.tree.get(trait_id);
        assert_eq!(tr.name, Some(parser.strings.intern("Baz")));
        let params = tr
            .static_parameters
            .as_ref()
            .expect("expected static params");
        assert_eq!(params.len(), 1);
        assert_eq!(tr.withs.len(), 1);

        // with T: Copy
        let with_id = tr.withs[0];
        let with = parser.tree.get(with_id);
        assert_eq!(with.clauses.len(), 1);
        match parser.tree.get(with.clauses[0]) {
            crate::WithClause::Assertion { target, assertion } => {
                match parser.tree.get(*target) {
                    Type::Path { path, .. } => {
                        assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
                    }
                    _ => panic!("expected path type for target"),
                }
                match parser.tree.get(*assertion) {
                    Type::Path { path, .. } => {
                        assert_eq!(
                            *path,
                            parser.paths.intern(vec![parser.strings.intern("Copy")])
                        );
                    }
                    _ => panic!("expected path type for assertion"),
                }
            }
            _ => panic!("expected assertion clause"),
        }
        assert_eq!(tr.functions.len(), 1);

        // function baz() => T
        let func_id = tr.functions[0];
        let func = parser.tree.get(func_id);
        let ret = func.return_type.expect("expected return type");
        match parser.tree.get(ret) {
            Type::Path { path, .. } => {
                assert_eq!(*path, parser.paths.intern(vec![parser.strings.intern("T")]));
            }
            _ => panic!("expected path type"),
        }
    }
}
