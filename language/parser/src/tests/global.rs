use crate::{ExpressionPosition, ExpressionStop};
use tspp_dir::{Declaration, Expression, GlobalDeclaration};

use crate::{TestParser, assert_node};

#[test]
fn test_parse_declare_global_block() {
    let test = TestParser::new(
        r###"
declare global {
    interface Foo {
        bar(): boolean
    }
}
"###,
    );
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
            assert!(*is_ambient);
            assert_eq!(expressions.len(), 1);
        });
    });
}

#[test]
fn test_parse_global_block_without_declare() {
    let test = TestParser::declaration(
        r###"
global {
    interface Foo { }
}
"###,
    );
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
            assert!(*is_ambient);
            assert_eq!(expressions.len(), 1);
        });
    });
}

#[test]
fn test_parse_global_declaration() {
    let test = TestParser::new(
        r###"
global {
    let process: Process
}
"###,
    );
    let mut parser = test.prepare();

    let expressions = parser.parse_in_place();
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
            assert!(!*is_ambient);
            assert_eq!(expressions.len(), 1);
        });
    });
}
