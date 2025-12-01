use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_ast::{
    Asynchrony, Declaration, DeclarationDescriptor, FunctionAbstraction, FunctionCardinality,
    FunctionKind, FunctionMode, FunctionSignature, Generics, Keyword, LocalNodeId, NodeType,
    Parameter, TokenType,
};

/// The keywords that can appear before a function declaration.
pub static FUNCTION_MODIFIERS: [Keyword; 7] = [
    Keyword::Async,
    Keyword::Abstract,
    Keyword::Override,
    Keyword::Get,
    Keyword::Set,
    Keyword::Constructor,
    Keyword::New,
];

impl Parser {
    /// Eat a function or "lambda" declaration or declaration.
    /// If no body is provided, it is a declaration for a function defined elsewhere.
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
    ///
    /// function foo<T, U>(x: T) => (int32, boolean) where (
    ///    T: Copy
    ///    U: Numeric
    /// ) {
    ///    print("Hello, world!")
    /// }
    ///
    /// function baz(a: int32, b: boolean) => (
    ///    MyStruct,
    ///    boolean
    /// ) with Disk, Time { // with can be on next line
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
        mut descriptor: DeclarationDescriptor,
        expect_maybe: bool,
        expect_body: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let start = self.mark();

        // async
        let is_async = if self.peek_keyword(Keyword::Async).is_ok() {
            self.bump(); // eat async keyword
            true
        } else {
            false
        };

        // new
        let mode = if self.peek_keyword(Keyword::New).is_ok()
            && (self.peek_next_token(TokenType::LessThan).is_ok()
                || self.peek_next_token(TokenType::OpenParenthesis).is_ok())
        {
            self.bump(); // eat new keyword
            Some(FunctionMode::New)
        } else {
            None
        };

        // function style
        let is_generator = self.eat_token_maybe(TokenType::Multiply)?;
        let (kind, is_generator) = {
            // regular `function` style
            if self.peek_keyword(Keyword::Function).is_ok() {
                self.bump(); // eat function keyword
                let is_generator = is_generator || self.eat_token_maybe(TokenType::Multiply)?;
                (FunctionKind::Function, is_generator)
            }
            // lambda style
            else {
                (FunctionKind::Lambda, is_generator)
            }
        };

        // function style, name, static parameters
        let (name, static_parameters) = {
            if kind == FunctionKind::Function {
                // name
                let name = self.eat_name_maybe()?;

                // maybe keyword after name (maybe)
                if expect_maybe {
                    self.eat_token(TokenType::Maybe)?;
                }

                // static parameters
                let static_parameters = self
                    .eat_static_parameters_maybe()
                    .for_node_type(NodeType::Declaration)?;

                (name, static_parameters)
            } else {
                // static parameters
                let static_parameters = self
                    .eat_static_parameters_maybe()
                    .for_node_type(NodeType::Declaration)?;

                (None, static_parameters)
            }
        };
        descriptor = descriptor.with_name_maybe(name);

        // dynamic parameters
        let dynamic_parameters = {
            // regular `(...) => ...` function/lambda
            if kind == FunctionKind::Function
                || self.options.in_type
                || self.peek_token(TokenType::OpenParenthesis).is_ok()
            {
                self.eat_token(TokenType::OpenParenthesis)?;
                self.eat_newlines_maybe()?;

                // dynamic parameters
                let dynamic_parameters = if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    vec![]
                } else {
                    self.eat_parameters_body()?
                };
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseParenthesis)?;

                dynamic_parameters
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

                vec![parameter_id]
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
                let return_type = self.with_options(self.options.nested().in_type(), |parser| {
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
                    let return_type = self.with_options(
                        self.options.nested().in_type().in_before_block(),
                        |parser| parser.eat_expression(),
                    )?;
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
                return Err(ParseError::expected(
                    self.peek()?.span,
                    TokenType::OpenBrace,
                ));
            }
            // function with body
            if kind == FunctionKind::Function && self.peek_token(TokenType::OpenBrace).is_ok() {
                let body = self.with_options(self.options.in_statement_position(), |parser| {
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
                let body = self.with_options(self.options.in_statement_position(), |parser| {
                    parser.eat_expression()
                })?;
                Some(body)
            }
            // no body
            else {
                None
            }
        };

        // function
        let generics = Generics::new(static_parameters, with_clauses, where_clauses).into_option();
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
        let signature = FunctionSignature {
            abstraction: FunctionAbstraction::Concrete,
            asynchrony,
            cardinality,
            kind,
            mode,
            generics,
            dynamic_parameters,
            return_type,
        };
        let function_id = self.tree.insert(
            Declaration::Function {
                descriptor,
                signature,
                body,
            },
            self.get_span_from(start),
        );
        Ok(function_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Asynchrony, BinaryOperator, Declaration, DeclarationDescriptor, Expression,
        FunctionCardinality, FunctionKind, FunctionMode, IntType, Parameter, TypeLiteral,
        WhereClause, WithClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_function_lambda_with_newlines() {
        let mut test = TestParser::new("(x: number):\n\tnumber =>\n\tx");
        let mut parser = test.prepare();

        let function_id = parser
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        // (x: number): number => x
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, body: Some(body), .. } => {
            assert_eq!(descriptor.name, None);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // x: number
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            });
            // number
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
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
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        // (x): int32 => x
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, body: Some(body), .. } => {
            assert_eq!(descriptor.name, None);
            assert_eq!(signature.kind, FunctionKind::Lambda);
            // x
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                assert_string!(parser, *name, "x");
            });
            // int32
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            // x
            assert_node!(parser.tree, *body, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
        });
    }

    #[test]
    fn test_parse_function_new_type() {
        let mut test = TestParser::new("new(): $");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        let function_id = parser
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        // new (x) => int32
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert!(descriptor.name.is_none());
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            assert!(signature.dynamic_parameters.is_empty());
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "$");
        });
    }

    #[test]
    fn test_parse_function_new_type_with_static_arguments() {
        let mut test = TestParser::new("new <T>(x: int32) => T");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        let function_id = parser
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        // new <T>(x: int32) => T
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert!(descriptor.name.is_none());
            // new
            assert_eq!(signature.mode, Some(FunctionMode::New));
            assert_eq!(signature.kind, FunctionKind::Lambda);
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            // <T>
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "T");
                assert!(ty.is_none());
            });
            // x: int32
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
            // T
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
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
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // function name
            assert_string!(parser, descriptor.name.unwrap().string(), "foo");
            let generics = signature.generics.as_ref().expect("expected generics");
            let with_clauses = generics.with_clauses.as_ref().unwrap();
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
            let where_clauses = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expression_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expression_path!(parser, parser.tree.get(*right), "Limit");
                });
            });
            // return type
            let ret = signature.return_type.expect("expected return type");
            assert_node!(parser.tree, ret, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
        });
    }

    #[test]
    fn test_parse_function_with_static_and_dynamic_parameters() {
        let mut test = TestParser::new(
            r"
function compute<Validate: boolean, Precision: uint8>(data: uint8[]) {
    body
}
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let function_id = parser
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // compute
            assert_string!(parser, descriptor.name.unwrap().string(), "compute");
            let static_parameters = signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .expect("expected static parameters");
            // <Validate: bool, Precision: uint8>
            assert_eq!(static_parameters.len(), 2);

            // Validate: bool
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Validate");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
            });

            // Precision: uint8
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "Precision");
                assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(8), is_signed: false })));
            });
            // data: uint8[]
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
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
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // foo
            assert_string!(parser, descriptor.name.unwrap().string() , "foo");

            // (str: string) => boolean
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.dynamic_parameters.len(), 1);
                    // str: string
                    assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                    });

                    // boolean
                    assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Boolean));
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
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        // async function* foo() => int32 { body }
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            // foo
            assert_string!(parser, descriptor.name.unwrap().string(), "foo");
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.cardinality, FunctionCardinality::Generator);
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
            .eat_function(DeclarationDescriptor::default(), false, false)
            .unwrap();
        assert_node!(parser.tree, function_id, Declaration::Function { descriptor, signature, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "onResolve");
            // callback: (args) => { .. } | void
            assert_eq!(signature.dynamic_parameters.len(), 1);
            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                assert_string!(parser, *name, "callback");
                assert_node!(parser.tree, ty.unwrap(), Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                        // args
                        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                            assert_string!(parser, *name, "args");
                        });
                        // { .. } | void
                        assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Binary { operator, left, right } => {
                            // { .. }
                            assert_node!(parser.tree, *left, Expression::ObjectExpression { ty: None, properties } => {
                                assert_eq!(properties.len(), 2);
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
            assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
        });
    }
}
