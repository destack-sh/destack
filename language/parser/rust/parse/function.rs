use dyst_ast::{
    Asynchrony, DefinitionMeta, FunctionAbstraction, FunctionCardinality, FunctionMode, NodeType,
    Parameter,
};

use crate::parse::prelude::*;
use crate::{ReferenceType, ScopedMutability, TokenType};

use crate::{
    Definition, FunctionKind, Keyword, Mutability, NodeId, Parser, ParserResult, Runtime,
    SelfParameter,
};

/// The keywords that can appear before a function definition.
pub static FUNCTION_MODIFIERS: [Keyword; 7] = [
    Keyword::Async,
    Keyword::Abstract,
    Keyword::Override,
    Keyword::Get,
    Keyword::Set,
    Keyword::Constructor,
    Keyword::New,
];

impl<'a> Parser<'a> {
    /// Peek a self keyword (also accepts `this`).
    fn peek_self_keyword(&mut self) -> ParserResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            Ok(keyword)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a self keyword (also accepts `this`).
    fn eat_self_keyword(&mut self) -> ParserResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keyword == Keyword::Self_ || keyword == Keyword::This {
            self.bump(); // eat self
            Ok(keyword)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat the self parameter maybe.
    fn eat_self_parameter_maybe(&mut self) -> ParserResult<Option<SelfParameter>> {
        let (keyword, mutability, reference_type) = {
            // var_ self
            if self.peek_keyword(Keyword::Var).is_ok()
                || self.peek_keyword(Keyword::Mut).is_ok()
                || self.peek_keyword(Keyword::Const).is_ok()
            {
                let mutability = self.eat_scoped_mutability()?;
                let keyword = self.eat_self_keyword()?; // eat self
                (keyword, mutability, None)
            }
            // self
            else if self.peek_self_keyword().is_ok() {
                let keyword = self.eat_self_keyword()?; // eat self
                let mutability = ScopedMutability::Unscoped {
                    mutability: Mutability::Immutable,
                };
                (keyword, mutability, None)
            }
            // &var_ self
            else if (self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::ElementwiseAnd).is_ok())
                && (self.peek_next_keyword(Keyword::Var).is_ok()
                    || self.peek_next_keyword(Keyword::Mut).is_ok()
                    || self.peek_next_keyword(Keyword::Const).is_ok())
            {
                self.bump(); // eat &
                let mutability = self.eat_scoped_mutability()?;
                let keyword = self.eat_self_keyword()?; // eat self
                (keyword, mutability, Some(ReferenceType::Reference))
            }
            // &self
            else if self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::ElementwiseAnd).is_ok()
            {
                self.bump(); // eat &
                let keyword = self.eat_self_keyword()?; // eat self
                let mutability = ScopedMutability::Unscoped {
                    mutability: Mutability::Immutable,
                };
                (keyword, mutability, Some(ReferenceType::Reference))
            }
            // ^var_ self
            else if self.peek_token(TokenType::ElementwiseXor).is_ok()
                && (self.peek_next_keyword(Keyword::Var).is_ok()
                    || self.peek_next_keyword(Keyword::Mut).is_ok()
                    || self.peek_next_keyword(Keyword::Const).is_ok())
            {
                self.bump(); // eat &
                let mutability = self.eat_scoped_mutability()?;
                let keyword = self.eat_self_keyword()?; // eat self
                (keyword, mutability, Some(ReferenceType::Value))
            }
            // ^self
            else if self.peek_token(TokenType::ElementwiseXor).is_ok() {
                self.bump(); // eat &
                let keyword = self.eat_self_keyword()?; // eat self
                let mutability = ScopedMutability::Unscoped {
                    mutability: Mutability::Immutable,
                };
                (keyword, mutability, Some(ReferenceType::Value))
            }
            // other
            else {
                return Ok(None);
            }
        };

        // type
        let ty = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let ty = self.with_options(self.options.in_type(), |parser| parser.eat_expression())?;
            Some(ty)
        } else {
            None
        };

        // self parameter
        Ok(Some(SelfParameter {
            keyword,
            mutability,
            reference_type,
            ty,
        }))
    }

    /// Eat a function or "lambda" definition or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
    /// If no function keyword is provided, it is a shorthand regular function or a lambda.
    /// (Lambda functions cannot have a name, runtime, or static parameters.)
    ///
    /// Examples:
    /// ```
    /// // lambda style (type context)
    /// (a: int32) => int32
    /// (int32) => (boolean, int32)
    /// (x): int32 => x
    ///
    /// // lambda style (value context)
    /// (a) => a > 2
    /// (a): int32 => a > 2
    /// (a: int32) => {
    ///    print("Hello, world!")
    /// }
    ///
    /// // function style
    /// function () // anonymous function with empty signature
    ///
    /// function foo() // just declaration, no body, no opening `{`
    /// foo()
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    /// foo<T, U>(x: T) => (int32, boolean) ... // shorthand
    ///
    /// function baz(a: int32, b: boolean) => (
    ///    MyStruct,
    ///    boolean
    /// ) with Disk, Time { // with can be on next line
    ///    ...
    /// }
    ///
    /// // getter/setter style
    /// get foo() => int32
    /// set foo(value: int32)
    ///
    /// function @comptime() {
    ///    ...
    /// }
    ///
    /// // optional , if newline-delimited
    /// function longBar<Validate: boolean>(
    ///   /// doc comment for `a`
    ///   a: int32
    ///   /// doc comment for `b`
    ///   b: boolean
    ///   // regular comment
    ///   c: Vector2
    /// ) => (
    ///    int32,
    ///    isGood: boolean
    /// ) with (
    ///   Time
    /// ) {
    ///    ...
    /// }
    /// ```
    pub fn eat_function(
        &mut self,
        mut meta: DefinitionMeta,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // abstraction
        let abstraction = if self.peek_keyword(Keyword::Abstract).is_ok() {
            self.bump(); // eat abstract keyword
            if self.peek_keyword(Keyword::Override).is_ok() {
                self.bump(); // eat override keyword
                FunctionAbstraction::AbstractOverride
            } else {
                FunctionAbstraction::Abstract
            }
        } else if self.peek_keyword(Keyword::Override).is_ok() {
            self.bump(); // eat override keyword
            FunctionAbstraction::ConcreteOverride
        } else {
            FunctionAbstraction::Concrete
        };

        // async
        let is_async = if self.peek_keyword(Keyword::Async).is_ok() {
            self.bump(); // eat async keyword
            true
        } else {
            false
        };

        // mode
        let mut mode = {
            // getter
            if self.peek_keyword(Keyword::Get).is_ok()
                && self.peek_next_token(TokenType::Identifier).is_ok()
            {
                self.bump(); // eat get keyword
                Some(FunctionMode::Getter)
            }
            // setter
            else if self.peek_keyword(Keyword::Set).is_ok()
                && self.peek_next_token(TokenType::Identifier).is_ok()
            {
                self.bump(); // eat set keyword
                Some(FunctionMode::Setter)
            }
            // constructor
            else if self.peek_keyword(Keyword::Constructor).is_ok()
                && (self.peek_next_token(TokenType::LessThan).is_ok()
                    || self.peek_next_token(TokenType::OpenParenthesis).is_ok())
            {
                self.bump(); // eat constructor keyword
                Some(FunctionMode::Constructor)
            }
            // new constructor
            else if self.peek_keyword(Keyword::New).is_ok()
                && (self.peek_next_token(TokenType::LessThan).is_ok()
                    || self.peek_next_token(TokenType::OpenParenthesis).is_ok())
            {
                self.bump(); // eat new keyword
                Some(FunctionMode::New)
            } else {
                None
            }
        };

        // function style
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;
        let (kind, is_generator) = {
            // constructors are always functions
            if mode == Some(FunctionMode::Constructor) {
                (FunctionKind::Function, false)
            }
            // regular `function` style
            else if self.peek_keyword(Keyword::Function).is_ok() {
                self.bump(); // eat function keyword
                let is_generator = is_generator || self.eat_token_maybe(TokenType::Multiply)?;
                (FunctionKind::Function, is_generator)
            }
            // shorthand `name()` or `name?()` style
            // (or `[Symbol.iterator]()` or `[Symbol.iterator]?()` style)
            else if self.peek_token(TokenType::Identifier).is_ok()
                && (!expect_maybe
                    && self
                        .peek_next_token_in(&[TokenType::LessThan, TokenType::OpenParenthesis])
                        .is_ok()
                    || expect_maybe
                        && self
                            .peek_next_next_token_in(&[
                                TokenType::LessThan,
                                TokenType::OpenParenthesis,
                            ])
                            .is_ok())
                || self.peek_token(TokenType::OpenBracket).is_ok()
            {
                (FunctionKind::Function, is_generator)
            }
            // lambda style
            else {
                (FunctionKind::Lambda, is_generator)
            }
        };

        // function style: runtime, name, static parameters
        let (runtime, name_or_key, static_parameters) = {
            if kind == FunctionKind::Function {
                // runtime
                let runtime = if self.peek_token(TokenType::At).is_ok() {
                    self.eat_token(TokenType::At)?;
                    Runtime::Static
                } else {
                    Runtime::Dynamic
                };

                // name
                let name_or_key = if self.peek_name_or_key().is_ok() {
                    Some(self.eat_name_or_key()?)
                } else {
                    None
                };

                // maybe keyword after name (maybe)
                if expect_maybe {
                    self.eat_token(TokenType::Maybe)?;
                }

                // static parameters
                let static_parameters = self
                    .eat_static_parameters_maybe()
                    .for_node_type(NodeType::Definition)?;

                (runtime, name_or_key, static_parameters)
            } else {
                // static parameters
                let static_parameters = self
                    .eat_static_parameters_maybe()
                    .for_node_type(NodeType::Definition)?;

                (Runtime::Dynamic, None, static_parameters)
            }
        };
        meta = meta.with_name_or_key_maybe(name_or_key);

        // dynamic parameters
        let (self_parameter, dynamic_parameters) = {
            // regular `(...) => ...` function/lambda
            if kind == FunctionKind::Function
                || self.options.in_type
                || self.peek_token(TokenType::OpenParenthesis).is_ok()
            {
                self.eat_token(TokenType::OpenParenthesis)?;
                self.eat_newlines_maybe()?;

                // self parameter maybe
                let self_parameter = self.eat_self_parameter_maybe()?;
                // optional separator after self before other parameters
                if self_parameter.is_some() && self.peek_item_stop().is_ok() {
                    self.eat_item_stop_with_newlines()?;
                }

                // other dynamic parameters
                let dynamic_parameters = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    vec![]
                } else {
                    self.eat_parameters_body()?
                };
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseParenthesis)?;

                (self_parameter, dynamic_parameters)
            }
            // simple no-parentheses `x => y` lambda value
            else {
                let parameter_name = self.eat_identifier()?;
                let parameter_id = self.tree.insert(
                    Parameter::Named {
                        modifiers: None,
                        name: parameter_name,
                        ty: None,
                        default: None,
                    },
                    self.get_span_from(start),
                );

                (None, vec![parameter_id])
            }
        };

        // return type info (including with/where)
        // only for functions or lambda types
        let (return_type, with_clauses, where_clauses) = {
            // lambda with explicit return type
            if kind == FunctionKind::Lambda
                && (self.peek_colon().is_ok() || self.options.in_type && self.peek_arrow().is_ok())
            {
                self.bump(); // eat colon or arrow
                self.eat_newlines_maybe()?;

                // return type
                let return_type = self.with_options(self.options.nested_in_type(), |parser| {
                    parser.eat_expression()
                })?;

                // with
                let with_clauses = self.eat_with_header_maybe()?;

                // where
                let where_clauses = self.eat_where_maybe()?;

                (Some(return_type), with_clauses, where_clauses)
            }
            // regular function with return type or lambda type
            else if kind == FunctionKind::Function || self.options.in_type {
                // return type
                let return_type = if self.peek_arrow().is_ok() || self.peek_colon().is_ok() {
                    self.bump(); // eat arrow or colon
                    self.eat_newlines_maybe()?;
                    let return_type = self
                        .with_options(self.options.nested_type_in_before_block(), |parser| {
                            parser.eat_expression()
                        })?;
                    Some(return_type)
                } else {
                    None
                };

                // with
                let with_clauses = self.eat_with_header_maybe()?;

                // where
                let where_clauses = self.eat_where_maybe()?;

                (return_type, with_clauses, where_clauses)
            }
            // nothing
            else {
                (None, None, None)
            }
        };

        // body
        // only for functions or lambda values
        let body = {
            // expect body but no opening brace
            if expect_body && self.peek_token(TokenType::OpenBrace).is_err() {
                return Err(ParserError::expected(
                    self.peek()?.span,
                    TokenType::OpenBrace,
                ));
            }
            // function with body
            if kind == FunctionKind::Function && self.peek_token(TokenType::OpenBrace).is_ok() {
                let body = self.with_options(self.options.in_block_position(), |parser| {
                    parser.eat_expression()
                })?;
                Some(body)
            }
            // lambda with body
            else if kind == FunctionKind::Lambda
                && !self.options.in_type
                && self.peek_arrow().is_ok()
            {
                self.eat_arrow()?;
                self.eat_newlines_maybe()?;
                let body = self.with_options(self.options.in_block_position(), |parser| {
                    parser.eat_expression()
                })?;
                Some(body)
            }
            // no body
            else {
                None
            }
        };

        // turn into call if we don't have a name or body
        if meta.name.is_none() && mode.is_none() && body.is_none() {
            mode = Some(FunctionMode::Call);
        }

        // function
        let asynchrony = if is_async {
            Asynchrony::Async
        } else {
            Asynchrony::Sync
        };
        let cardinality = if is_generator {
            FunctionCardinality::Generator
        } else {
            FunctionCardinality::Scalar
        };
        let function_id = self.tree.insert(
            Definition::Function {
                meta,
                runtime,
                abstraction,
                asynchrony,
                cardinality,
                mode,
                kind,
                static_parameters,
                self_parameter,
                dynamic_parameters,
                return_type,
                with_clauses,
                where_clauses,
                body,
            },
            self.get_span_from(start),
        );
        Ok(function_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Asynchrony, Block, DefinitionMeta, FunctionCardinality, FunctionMode, FunctionKind,
        Parameter, ReferenceType,
    };

    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, Mutability, ScopedMutability, TypeLiteral,
        WhereClause, WithClause, assert_expr_path, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_function_lambda_with_newlines() {
        let mut test = TestParser::new("(x: number):\n\tnumber =>\n\tx");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        // (x: number): number => x
        assert_node!(parser.tree, function_id, Definition::Function { meta, kind, dynamic_parameters, return_type, body: Some(body), .. } => {
            assert_eq!(meta.name, None);
            assert_eq!(*kind, FunctionKind::Lambda);
            // x: number
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            });
            // number
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    #[test]
    fn test_parse_function_lambda_with_explicit_return_type() {
        let mut test = TestParser::new("(x): int32 => x");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        // (x): int32 => x
        assert_node!(parser.tree, function_id, Definition::Function { meta, kind, dynamic_parameters, return_type, body: Some(body), .. } => {
            assert_eq!(meta.name, None);
            assert_eq!(*kind, FunctionKind::Lambda);
            // x
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // int32
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    #[test]
    fn test_parse_function_lambda_call_expression() {
        let mut test = TestParser::new("<T = any>(x: T): T");
        let mut parser = test.prepare();
        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        // <T = any>(x: T): T
        assert_node!(parser.tree, function_id, Definition::Function { meta, mode, kind, static_parameters, dynamic_parameters, return_type, .. } => {
            assert_eq!(meta.name, None);
            assert_eq!(*kind, FunctionKind::Lambda);
            assert_eq!(*mode, Some(FunctionMode::Call));
            // <T = any>
            assert_eq!(static_parameters.as_ref().unwrap().len(), 1);
            assert_node!(parser.tree, static_parameters.as_ref().unwrap()[0], Parameter::Named { name, ty: None, default, .. } => {
                assert_string!(parser, *name, "T");
                assert_node!(parser.tree, default.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
            });
            // x: T
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_expr_path!(parser, parser.tree.get(ty.unwrap()), "T");
            });
            // T
            assert_expr_path!(parser, parser.tree.get(return_type.unwrap()), "T");
        });
    }

    #[test]
    fn test_parse_function_lambda_type_constructor_expression() {
        let mut test = TestParser::new("new() => $");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        let expression_id = parser.eat_expression().unwrap();
        // new (x) => int32
        assert_node!(parser.tree, expression_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function { meta, kind, mode, static_parameters, dynamic_parameters, return_type, .. } => {
                assert_eq!(meta.name, None);
                assert_eq!(*mode, Some(FunctionMode::New));
                assert_eq!(*kind, FunctionKind::Lambda);
                assert!(static_parameters.is_none());
                assert!(dynamic_parameters.is_empty());
                assert_expr_path!(parser, parser.tree.get(return_type.unwrap()), "$");
            });
        });
    }

    #[test]
    fn test_parse_function_lambda_type_constructor_expression_with_static_arguments() {
        let mut test = TestParser::new("new <T>(x: int32) => T");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        let expression_id = parser.eat_expression().unwrap();
        // new <T>(x: int32) => T
        assert_node!(parser.tree, expression_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function { meta, kind, mode, static_parameters, dynamic_parameters, return_type, .. } => {
                assert_eq!(meta.name, None);
                assert_eq!(*mode, Some(FunctionMode::New));
                assert_eq!(*kind, FunctionKind::Lambda);

                // <T>
                assert_eq!(static_parameters.as_ref().unwrap().len(), 1);
                assert_node!(parser.tree, static_parameters.as_ref().unwrap()[0], Parameter::Named { name, .. } => {
                    assert_string!(parser, *name, "T");
                });

                // x: int32
                assert_eq!(dynamic_parameters.len(), 1);
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                        assert_eq!(int_ty.width, Some(32));
                        assert!(int_ty.is_signed);
                    });
                });

                // T
                assert_expr_path!(parser, parser.tree.get(return_type.unwrap()), "T");
            });
        });
    }

    #[test]
    fn test_parse_function_shorthand_style() {
        let mut test = TestParser::new("foo() => int32 { body }");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, kind, dynamic_parameters, return_type,  .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "foo");
            assert_eq!(*kind, FunctionKind::Function);
            // ()
            assert_eq!(dynamic_parameters.len(), 0);
            // int32
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_with_clause() {
        let mut test = TestParser::new(
            r###"
function foo() => int32 with (
  Time,
  F: Numeric,
) where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, with_clauses, where_clauses, return_type, .. } => {
            // function name
            assert_string!(parser, meta.name.unwrap().string(), "foo");

            let with_clauses = with_clauses.as_ref().unwrap();
            assert_eq!(with_clauses.len(), 2);

            // Time
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, with_clauses[1], WithClause { alias, right } => {
                assert_string!(parser, alias.unwrap(), "F");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Numeric");
                });
            });

            // where Guard > Limit
            let where_clauses = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expr_path!(parser, parser.tree.get(*right), "Limit");
                });
            });

            // return type
            let ret = return_type.expect("expected return type");
            assert_node!(parser.tree, ret, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                assert_eq!(int_ty.width, Some(32));
                assert!(int_ty.is_signed);
            });
        });
    }

    #[test]
    fn test_parse_function_self_parameter_simple() {
        let mut test = TestParser::new("function a(self) {}");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "a");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert_eq!(self_param.reference_type, None);
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert!(dynamic_parameters.is_empty());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_self_parameter_pointer() {
        let mut test = TestParser::new(
            r###"
function b(
  &self
  x: int32
) {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "b");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert_eq!(self_param.reference_type, Some(ReferenceType::Reference));
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });

            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                let param_type = ty.expect("expected type for parameter x");
                assert_node!(parser.tree, param_type, Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                    assert_eq!(int_ty.width, Some(32));
                    assert!(int_ty.is_signed);
                });
            });
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_self_parameter_mutable_pointer() {
        let mut test = TestParser::new("function c(&var self) {}");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, self_parameter, dynamic_parameters, where_clauses, .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "c");

            let self_param = self_parameter.as_ref().expect("expected self param");
            assert_eq!(self_param.reference_type, Some(ReferenceType::Reference));
            assert_eq!(self_param.mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });

            assert!(dynamic_parameters.is_empty());
            assert!(where_clauses.is_none());
        });
    }

    #[test]
    fn test_parse_function_with_static_and_dynamic_parameters() {
        let mut test = TestParser::new(
            r"
function compute<Validate: bool, Precision: uint8>(data: uint8[]) {
    body
}
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, static_parameters, dynamic_parameters, .. } => {
            // compute
            assert_string!(parser, meta.name.unwrap().string(), "compute");

            // <Validate: bool, Precision: uint8>
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 2);

            // Validate: bool
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Validate");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            });

            // Precision: uint8
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Precision");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(int_ty)) => {
                    assert_eq!(int_ty.width, Some(8));
                    assert!(!int_ty.is_signed);
                });
            });

            // data: uint8[]
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "data");
            });
        });
    }

    #[test]
    fn test_parse_function_with_function_return_type() {
        let mut test = TestParser::new("function foo() => (str: string) => boolean {}");
        let mut parser = test.prepare();

        // function foo() => (str: string) => boolean
        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, return_type, .. } => {
            // foo
        assert_string!(parser, meta.name.unwrap().string() , "foo");

            // (str: string) => boolean
            assert_node!(parser.tree, return_type.unwrap(), Expression::Definition(definition_id) => {
                assert_node!(parser.tree, *definition_id, Definition::Function { dynamic_parameters, return_type, .. } => {
                    assert_eq!(dynamic_parameters.len(), 1);
                    // str: string
                    assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    });

                    // boolean
                    assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
                });
            });
        });
    }

    #[test]
    fn test_parse_function_with_async_generator() {
        let mut test = TestParser::new(
            r#"
async function* foo() => int32 { 
    yield 1
    yield 2
    yield 3
}"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        // async function* foo() => int32 { body }
        assert_node!(parser.tree, function_id, Definition::Function { meta, asynchrony, cardinality, .. } => {
            // foo
            assert_string!(parser, meta.name.unwrap().string(), "foo");
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(*cardinality, FunctionCardinality::Generator);
        });
    }

    #[test]
    fn test_parse_function_with_symbol_iterator() {
        let mut test = TestParser::new(
            r#"
async *[Symbol.iterator]() {
    yield 1; 
    yield 2; 
    yield 3;
}
            "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, asynchrony, cardinality, body, .. } => {
            assert!(meta.name.is_none());
            assert_expr_path!(parser, parser.tree.get(meta.key.unwrap()), "Symbol.iterator");
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_eq!(*cardinality, FunctionCardinality::Generator);
            assert_node!(parser.tree, body.unwrap(), Expression::Block(block) => {
                assert_node!(parser.tree, *block, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 3);
                });
            });
        });
    }

    #[test]
    fn test_parse_function_constructor() {
        let mut test = TestParser::new("constructor(x: int32);");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();

        assert_node!(parser.tree, function_id, Definition::Function { meta, mode, dynamic_parameters, body, .. } => {
            assert!(meta.name.is_none());
            assert_eq!(*mode, Some(FunctionMode::Constructor));
            assert_eq!(dynamic_parameters.len(), 1);
            assert!(body.is_none());
        });
    }

    #[test]
    fn test_parse_function_with_nested_lambda_type() {
        let mut test = TestParser::new(
            r#"
function onResolve(
    callback: (args) => {
        path: string;
        namespace?: string;
    } | void,
) => void;
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DefinitionMeta::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Definition::Function { meta, dynamic_parameters, return_type, .. } => {
            assert_string!(parser, meta.name.unwrap().string(), "onResolve");
            // callback: (args) => { .. } | void
            assert_eq!(dynamic_parameters.len(), 1);
            assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "callback");
                assert_node!(parser.tree, ty.unwrap(), Expression::Definition(definition_id) => {
                    assert_node!(parser.tree, *definition_id, Definition::Function { dynamic_parameters, return_type, .. } => {
                        assert_eq!(dynamic_parameters.len(), 1);
                        // args
                        assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                            assert_string!(parser, *name, "args");
                        });
                        // { .. } | void
                        assert_node!(parser.tree, return_type.unwrap(), Expression::Binary { operator, left, right } => {
                            // { .. }
                            assert_node!(parser.tree, *left, Expression::StructLiteral { ty: None, fields } => {
                                assert_eq!(fields.len(), 2);
                            });
                            // |
                            assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                            // void
                            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Void));
                        });
                    });
                });
            });
            // void
            assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
        });
    }
}
