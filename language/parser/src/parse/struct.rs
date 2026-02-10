#![allow(clippy::type_complexity)]

use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, DeclarationDescriptor, Generics, Heritage, Keyword, LocalNodeId, NodeType,
    TokenType,
};

impl Parser {
    /// Eat a struct or class declaration.
    ///
    /// The parser accepts `extends` for both, but structs cannot semantically
    /// use extends (use embedding instead). This is validated in the analyze phase.
    /// Struct declarations require a name.
    ///
    /// Struct examples:
    /// ```
    /// struct Bar {
    ///     myField: int32;
    ///     myOtherField: boolean;
    /// };
    ///
    /// struct Foo<T> implements Drawable { // structs can implement interfaces
    ///     myField: int32;
    ///     myOtherField: T;
    ///
    ///     ...Bar;              // embedding for composition
    ///     static x: int32 = 7; // constant
    ///
    ///     myFunc() { }
    /// };
    /// ```
    ///
    /// Class examples:
    /// ```
    /// class Foo extends Bar { // classes can extend
    ///     myField: int32;
    /// };
    /// ```
    pub fn eat_struct_or_class(
        &mut self,
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
        allow_anonymous_class: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_STRUCT);
        // keyword
        let keyword = self
            .eat_keyword_in(&[Keyword::Struct, Keyword::Class])
            .for_node_type(NodeType::Declaration)?;
        let is_class = keyword == Keyword::Class;

        // optional name / key
        let has_heritage_keyword = self.peek_keyword(Keyword::Extends).is_ok()
            || self.peek_keyword(Keyword::Implements).is_ok();

        // require a name for class and struct declarations
        let allow_anonymous = is_class && allow_anonymous_class;
        let name_span = if !allow_anonymous {
            let (name, span) = self.eat_name_with_span()?;
            descriptor = descriptor.with_name(name);
            Some(span)
        } else if has_heritage_keyword {
            None
        } else if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            descriptor = descriptor.with_name(name);
            Some(span)
        } else {
            None
        };

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Declaration)?;

        // optional extends types
        let extends_types = if is_class {
            self.eat_extends_expressions_maybe()
                .for_node_type(NodeType::Declaration)?
        } else {
            self.eat_extends_types_maybe()
                .for_node_type(NodeType::Declaration)?
        };

        // optional implements types
        let implements_types = self
            .eat_implements_types_maybe()
            .for_node_type(NodeType::Declaration)?;

        // where
        let where_clauses = self
            .eat_where_maybe()
            .for_node_type(NodeType::Declaration)?;

        // body
        self.eat_newlines_maybe()?;
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let members = self
            .with_options(self.options.nested().in_variant(), |parser| {
                parser.eat_members(false)
            })
            .for_node_type(NodeType::Declaration)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;

        // struct or class
        let generics = Generics::new(static_parameters, where_clauses);
        let heritage = Heritage::new(extends_types, implements_types);
        let declaration = if is_class {
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
            }
        } else {
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                members,
            }
        };
        let declaration_id = self.tree.insert(declaration, self.get_span_from(start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(declaration_id, span);
        }

        Ok(declaration_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        BinaryOperator, BindingKind, Declaration, DeclarationDescriptor, DeclarationKind,
        Expression, IntType, Key, Member, Name, Parameter, ScalarLiteral, TypeLiteral, Visibility,
        WhereClause,
    };
    use destack_source::{LanguageType, NodeSpanType};

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_struct_requires_name() {
        let mut test = TestParser::new(
            r###"
struct { public x: int32, readonly y: boolean }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_struct_or_class(&start, DeclarationDescriptor::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_class_requires_name_in_statement_position() {
        let mut test = TestParser::new(
            r###"
class {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_struct_or_class(&start, DeclarationDescriptor::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_class_rejects_comma_separated_members() {
        // source: class Foo { x: int32, y: int32 }
        let mut test = TestParser::new(
            r###"
class Foo { x: int32, y: int32 }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_struct_or_class(&start, DeclarationDescriptor::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_struct_rejects_comma_separated_members() {
        // source: struct Foo { x: int32, y: int32 }
        let mut test = TestParser::new(
            r###"
struct Foo { x: int32, y: int32 }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_struct_or_class(&start, DeclarationDescriptor::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_struct_with_extends_types() {
        let mut test = TestParser::new(
            r###"
struct Foo extends Bar {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct { descriptor, heritage, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(members.is_empty());
            assert!(!heritage.is_empty());
            assert!(heritage.implements_types.is_none());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_javascript_class_allows_parenthesized_binary_extends_expression() {
        let mut test =
            TestParser::new_with_options("class A extends (a + b) {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_expression_path!(parser, parser.tree.get(*left), "a");
                assert_expression_path!(parser, parser.tree.get(*right), "b");
            });
        });
    }

    #[test]
    fn test_parse_class_with_multiple_extends_for_lineage_validation() {
        let mut test = TestParser::new(
            r###"
class Combined extends First, Second {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class { descriptor, heritage, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "Combined");

            let extends_types = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(extends_types.len(), 2);

            assert_node!(parser.tree, extends_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "First");
            });
            assert_node!(parser.tree, extends_types[1], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Second");
            });
        });
    }

    #[test]
    fn test_parse_class_with_empty_extends_for_lineage_validation() {
        let mut test = TestParser::new(
            r###"
class Counter extends {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class { descriptor, heritage, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "Counter");

            let extends_types = heritage.extends_types.as_ref().expect("expected extends clause");
            assert!(extends_types.is_empty());
        });
    }

    #[test]
    fn test_parse_struct_with_spread() {
        let mut test = TestParser::new(
            r###"
struct Foo<T: Numeric> extends Boz implements Quux {
    ...Bar
    ...Baz

    a: T
    b?: T
    c: T
    private d: int32 = 4
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct { descriptor, generics, heritage, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(!generics.is_empty());

            // T: Numeric
            let static_parameters = generics
                .static_parameters
                .as_ref()
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                // T
                assert_string!(parser, *name, "T");
                // Numeric
                assert!(ty.is_some());
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Numeric");
                });
            });
            assert!(!heritage.is_empty());

            // Boz
            let extends_types = heritage.extends_types.as_ref().unwrap();
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Boz");
            });

            let implements_types = heritage
                .implements_types
                .as_ref()
                .expect("expected implements types");
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Quux");
            });

            assert_eq!(members.len(), 6);

            // ..Bar
            assert_node!(parser.tree, members[0], Member::Embed { modifiers: None, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Bar");
            });
            // ..Baz
            assert_node!(parser.tree, members[1], Member::Embed { modifiers: None, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Baz");
            });
            // a: T
            assert_node!(parser.tree, members[2], Member::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // b?: T
            assert_node!(parser.tree, members[3], Member::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_eq!(modifiers.kind.unwrap(), BindingKind::Maybe);
                assert_string!(parser, *name, "b");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });
            // c: T
            assert_node!(parser.tree, members[4], Member::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // private d: int32 = 4
            assert_node!(parser.tree, members[5], Member::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: Some(value), .. } => {
                assert_eq!(modifiers.visibility.unwrap(), Visibility::Private);
                assert_string!(parser, *name, "d");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_where_clause() {
        let mut test = TestParser::new(
            r###"
struct Foo where Guard: Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct { descriptor, generics, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(members.is_empty());
            assert!(!generics.is_empty());

            // where Guard: Limit
            let where_clauses = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });
        });
    }

    #[test]
    fn test_parse_struct_with_private_member_function() {
        let mut test = TestParser::new(
            r###"
struct Foo {
    private enqueue<M extends F<"mutation">>() {
        throw new Error("Not implemented");
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct { descriptor, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_eq!(members.len(), 1);
        });
    }

    #[test]
    fn test_parse_struct_heritage_type_spans() {
        let mut test = TestParser::new("struct Foo extends Bar.Baz implements Qux {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationDescriptor::default(), false)
            .unwrap();

        // spans on extends and implements types
        assert_node!(parser.tree, struct_id, Declaration::Struct { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends types");
            let extends_span = parser
                .tree
                .get_side_span(extends_types[0], NodeSpanType::Type)
                .expect("expected extends type span");
            assert_eq!(parser.get_span_str(extends_span), "Bar.Baz");

            let implements_types = heritage.implements_types.as_ref().expect("expected implements types");
            let implements_span = parser
                .tree
                .get_side_span(implements_types[0], NodeSpanType::Type)
                .expect("expected implements type span");
            assert_eq!(parser.get_span_str(implements_span), "Qux");
        });
    }
}
