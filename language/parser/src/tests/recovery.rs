use crate::tests::TestParser;
use crate::{assert_expression_path, assert_node};
use tspp_dir::{
    Argument, Declaration, Declarator, Expression, InterfaceDeclaration, NodeType, TokenType,
    TypeExpression, TypeMember,
};

#[test]
fn test_recover_declaration_after_missing_binding() {
    for head in ["declare const", "let", "using"] {
        let source = format!(
            r#"{head}
/// Marker documentation.
@languageItem("memory.Copy") export newtype interface Marker {{}}
"#
        );
        let test = TestParser::new(&source);
        let mut parser = test.prepare();
        let expressions = parser.parse_in_place();

        // preserve both root expressions
        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Error);

        // preserve the following declaration annotations
        assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
            assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
                assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
                assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
            });
        });

        // report the missing declarator at the declaration boundary
        test.assert_errors(&parser, &[(Some(NodeType::Declarator), None, None, "@")]);
    }
}

#[test]
fn test_recover_incomplete_type_before_decorated_declaration() {
    let test = TestParser::new(
        r#"const value:
/// Marker documentation.
@languageItem("memory.Copy") export newtype interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the incomplete binding
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_node!(parser.tree, declarators[0], Declarator { ty, .. } => {
            let ty = ty.expect("expected recovered type");
            assert_node!(parser.tree, ty, TypeExpression::Missing);
        });
    });

    // retain the decorated declaration
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(&parser, &[(Some(NodeType::Declarator), None, None, "@")]);
}

#[test]
fn test_parse_decorated_type_before_declaration() {
    let test = TestParser::new(
        r#"const value:
@(addrspace) &Buffer;
export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated type
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Let { declarators, .. } => {
        assert_node!(parser.tree, declarators[0], Declarator { ty, .. } => {
            let ty = ty.expect("expected declared type");
            assert_node!(parser.tree, ty, TypeExpression::BorrowedOf { target_type, .. } => {
                assert_expression_path!(parser, parser.tree.get(*target_type), "Buffer");
            });
            assert_eq!(parser.tree.get_decorators(ty.id).len(), 1);
        });
    });

    // retain the following declaration
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }));
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_recover_parameter_list_before_decorated_declaration() {
    let test = TestParser::new(
        r#"declare function broken(value:
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete function
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Parameter), None, None, "@"),
            (
                Some(NodeType::Parameter),
                Some(TokenType::At),
                Some(TokenType::CloseParenthesis),
                "@",
            ),
        ],
    );
}

#[test]
fn test_recover_parameter_default_before_decorated_declaration() {
    let test = TestParser::new(
        r#"declare function broken(value =
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete default
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Parameter), None, None, "@"),
            (
                Some(NodeType::Parameter),
                Some(TokenType::At),
                Some(TokenType::CloseParenthesis),
                "@",
            ),
        ],
    );
}

#[test]
fn test_recover_generic_constraint_before_decorated_declaration() {
    let test = TestParser::new(
        r#"declare function broken<Value:
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete constraint
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::GenericParameter), None, None, "@"),
            (
                Some(NodeType::Expression),
                Some(TokenType::At),
                Some(TokenType::GreaterThan),
                "@",
            ),
            (
                Some(NodeType::Declaration),
                Some(TokenType::At),
                Some(TokenType::OpenParenthesis),
                "@",
            ),
        ],
    );
}

#[test]
fn test_recover_return_type_before_decorated_declaration() {
    let test = TestParser::new(
        r#"declare function broken():
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete return type
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(&parser, &[(Some(NodeType::Declaration), None, None, "@")]);
}

#[test]
fn test_recover_argument_list_before_decorated_declaration() {
    let test = TestParser::new(
        r#"consume(
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete call
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::At),
            Some(TokenType::CloseParenthesis),
            "@",
        )],
    );
}

#[test]
fn test_parse_decorated_class_argument() {
    let test = TestParser::new(
        r#"consume(
    @sealed class Inline {}
)
"#,
    );
    let (parser, expressions) = test.parse();

    // retain the decorated class inside the call
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Call { arguments, .. } => {
        assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::Declaration(_));
            assert_eq!(parser.tree.get_decorators(arguments[0].id).len(), 1);
        });
    });
}

#[test]
fn test_recover_type_member_list_before_decorated_declaration() {
    let test = TestParser::new(
        r#"interface Broken {
    value:
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete interface
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::TypeMember), None, None, "@"),
            (
                Some(NodeType::Declaration),
                Some(TokenType::At),
                Some(TokenType::CloseBrace),
                "@",
            ),
        ],
    );
}

#[test]
fn test_parse_associated_members_after_incomplete_type_member() {
    let test = TestParser::new(
        r#"interface Broken {
    broken: ;
    const Value: string = "value"
    type Item = string
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the damaged field and both associated members inside the interface
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_eq!(members.len(), 3);
            assert_node!(parser.tree, members[0], TypeMember::Field { .. });
            assert_node!(parser.tree, members[1], TypeMember::AssociatedConst { .. });
            assert_node!(parser.tree, members[2], TypeMember::AssociatedType { .. });
        });
    });
    test.assert_errors(&parser, &[(Some(NodeType::TypeMember), None, None, ";")]);
}

#[test]
fn test_recover_member_list_before_decorated_declaration() {
    let test = TestParser::new(
        r#"class Broken {
    value:
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete class
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Member), None, None, "@"),
            (
                Some(NodeType::Declaration),
                Some(TokenType::At),
                Some(TokenType::CloseBrace),
                "@",
            ),
        ],
    );
}

#[test]
fn test_recover_enum_body_before_decorated_declaration() {
    let test = TestParser::new(
        r#"enum Broken {
    value:
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete enum
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Member), None, None, "@"),
            (
                Some(NodeType::Declaration),
                Some(TokenType::At),
                Some(TokenType::CloseBrace),
                "@",
            ),
        ],
    );
}

#[test]
fn test_recover_object_literal_before_decorated_declaration() {
    let test = TestParser::new(
        r#"const broken = {
    value: ;
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete object
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Property), None, None, ";"),
            (
                Some(NodeType::Expression),
                Some(TokenType::Semicolon),
                Some(TokenType::CloseBrace),
                ";",
            ),
        ],
    );
}

#[test]
fn test_recover_initializer_before_decorated_declaration() {
    let test = TestParser::new(
        r#"const broken =
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete initializer
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(&parser, &[(Some(NodeType::Declarator), None, None, "@")]);
}

#[test]
fn test_recover_member_initializer_before_decorated_declaration() {
    let test = TestParser::new(
        r#"class Broken {
    value =
/// Marker documentation.
@languageItem("memory.Copy") export interface Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete member initializer
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Interface(InterfaceDeclaration { .. }) => {
            assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
            assert_eq!(parser.tree.get_decorators(declaration.id).len(), 1);
        });
    });
    test.assert_errors(
        &parser,
        &[
            (Some(NodeType::Member), None, None, "@"),
            (
                Some(NodeType::Declaration),
                Some(TokenType::At),
                Some(TokenType::CloseBrace),
                "@",
            ),
        ],
    );
}

#[test]
fn test_recover_modified_decorated_declaration() {
    let test = TestParser::new(
        r#"const value:
/// Marker documentation.
@(outer) export @inner<string>("second") declare abstract class Marker {}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // retain the decorated declaration after the incomplete binding
    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration) => {
        assert_eq!(test.documentation(&parser, *declaration), Some("Marker documentation."));
        assert_eq!(parser.tree.get_decorators(declaration.id).len(), 2);
    });
    test.assert_errors(&parser, &[(Some(NodeType::Declarator), None, None, "@")]);
}
